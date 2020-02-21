use super::bits::{usizes_for_bits, BitChunkIter, BitMatrix, Pair, TRUE_CHUNK};
use super::vec_storage::VecStorage;
use crate::common::*;
use crate::runtime::endpoint::EndpointExt;
use crate::runtime::endpoint::EndpointInfo;
use crate::runtime::endpoint::{Endpoint, Msg, SetupMsg};
use crate::runtime::errors::EndpointErr;
use crate::runtime::errors::MessengerRecvErr;
use crate::runtime::errors::PollDeadlineErr;
use crate::runtime::MessengerState;
use crate::runtime::Messengerlike;
use crate::runtime::ReceivedMsg;
use crate::runtime::{ProtocolD, ProtocolS};

use std::net::SocketAddr;
use std::sync::Arc;

pub enum Coupling {
    Active,
    Passive,
}

#[derive(Debug)]
struct Family {
    parent: Option<Port>,
    children: HashSet<Port>,
}

pub struct Binding {
    pub coupling: Coupling,
    pub polarity: Polarity,
    pub addr: SocketAddr,
}

pub struct InPort(Port); // InPort and OutPort are AFFINE (exposed to Rust API)
pub struct OutPort(Port);
impl From<InPort> for Port {
    fn from(x: InPort) -> Self {
        x.0
    }
}
impl From<OutPort> for Port {
    fn from(x: OutPort) -> Self {
        x.0
    }
}

#[derive(Default, Debug)]
struct ChannelIndexStream {
    next: u32,
}
impl ChannelIndexStream {
    fn next(&mut self) -> u32 {
        self.next += 1;
        self.next - 1
    }
}

enum Connector {
    Connecting(Connecting),
    Connected(Connected),
}

#[derive(Default)]
pub struct Connecting {
    bindings: Vec<Binding>,
}
trait Binds<T> {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> T;
}
impl Binds<InPort> for Connecting {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> InPort {
        self.bindings.push(Binding { coupling, polarity: Polarity::Getter, addr });
        InPort(Port(self.bindings.len() - 1))
    }
}
impl Binds<OutPort> for Connecting {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> OutPort {
        self.bindings.push(Binding { coupling, polarity: Polarity::Putter, addr });
        OutPort(Port(self.bindings.len() - 1))
    }
}

#[derive(Debug, Clone)]
pub enum ConnectErr {
    BindErr(SocketAddr),
    NewSocketErr(SocketAddr),
    AcceptErr(SocketAddr),
    ConnectionShutdown(SocketAddr),
    PortKindMismatch(Port, SocketAddr),
    EndpointErr(Port, EndpointErr),
    PollInitFailed,
    PollingFailed,
    Timeout,
}

#[derive(Debug)]
struct Component {
    protocol: Arc<ProtocolD>,
    port_set: HashSet<Port>,
    identifier: Arc<[u8]>,
    state: Option<ProtocolS>, // invariant between rounds: Some()
}

impl From<PollDeadlineErr> for ConnectErr {
    fn from(e: PollDeadlineErr) -> Self {
        use PollDeadlineErr as P;
        match e {
            P::PollingFailed => Self::PollingFailed,
            P::Timeout => Self::Timeout,
        }
    }
}
impl From<MessengerRecvErr> for ConnectErr {
    fn from(e: MessengerRecvErr) -> Self {
        use MessengerRecvErr as M;
        match e {
            M::PollingFailed => Self::PollingFailed,
            M::EndpointErr(port, err) => Self::EndpointErr(port, err),
        }
    }
}
impl Connecting {
    fn random_controller_id() -> ControllerId {
        type Bytes8 = [u8; std::mem::size_of::<ControllerId>()];
        let mut bytes = Bytes8::default();
        getrandom::getrandom(&mut bytes).unwrap();
        unsafe {
            // safe:
            // 1. All random bytes give valid Bytes8
            // 2. Bytes8 and ControllerId have same valid representations
            std::mem::transmute::<Bytes8, ControllerId>(bytes)
        }
    }
    fn test_stream_connectivity(stream: &mut TcpStream) -> bool {
        use std::io::Write;
        stream.write(&[]).is_ok()
    }
    fn new_connected(
        &self,
        controller_id: ControllerId,
        timeout: Option<Duration>,
    ) -> Result<Connected, ConnectErr> {
        use ConnectErr::*;

        ///////////////////////////////////////////////////////
        // 1. bindings correspond with ports 0..bindings.len(). For each:
        //    - reserve a slot in endpoint_exts.
        //    - store the port in `native_ports' set.
        let mut endpoint_exts = VecStorage::<EndpointExt>::with_reserved_range(self.bindings.len());
        let native_ports = (0..self.bindings.len()).map(Port).collect();

        // 2. create MessengerState structure for polling channels
        let edge = PollOpt::edge();
        let [ready_r, ready_w] = [Ready::readable(), Ready::writable()];
        let mut ms =
            MessengerState::with_event_capacity(self.bindings.len()).map_err(|_| PollInitFailed)?;

        // 3. create one TODO task per (port,binding) as a vector with indices in lockstep.
        //    we will drain it gradually so we store elements of type Option<Todo> where all are initially Some(_)
        enum Todo {
            PassiveAccepting { listener: TcpListener, channel_id: ChannelId },
            ActiveConnecting { stream: TcpStream },
            PassiveConnecting { stream: TcpStream, channel_id: ChannelId },
            ActiveRecving { endpoint: Endpoint },
        }
        let mut channel_index_stream = ChannelIndexStream::default();
        let mut todos = self
            .bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| {
                Ok(Some(match binding.coupling {
                    Coupling::Passive => {
                        let channel_index = channel_index_stream.next();
                        let channel_id = ChannelId { controller_id, channel_index };
                        let listener =
                            TcpListener::bind(&binding.addr).map_err(|_| BindErr(binding.addr))?;
                        ms.poll.register(&listener, Token(index), ready_r, edge).unwrap(); // registration unique
                        Todo::PassiveAccepting { listener, channel_id }
                    }
                    Coupling::Active => {
                        let stream = TcpStream::connect(&binding.addr)
                            .map_err(|_| NewSocketErr(binding.addr))?;
                        ms.poll.register(&stream, Token(index), ready_w, edge).unwrap(); // registration unique
                        Todo::ActiveConnecting { stream }
                    }
                }))
            })
            .collect::<Result<Vec<Option<Todo>>, ConnectErr>>()?;
        let mut num_todos_remaining = todos.len();

        // 4. handle incoming events until all TODOs are completed OR we timeout
        let deadline = timeout.map(|t| Instant::now() + t);
        let mut polled_undrained_later = IndexSet::<_>::default();
        let mut backoff_millis = 10;
        while num_todos_remaining > 0 {
            ms.poll_events_until(deadline)?;
            for event in ms.events.iter() {
                let token = event.token();
                let index = token.0;
                let binding = &self.bindings[index];
                match todos[index].take() {
                    None => {
                        polled_undrained_later.insert(index);
                    }
                    Some(Todo::PassiveAccepting { listener, channel_id }) => {
                        let (stream, _peer_addr) =
                            listener.accept().map_err(|_| AcceptErr(binding.addr))?;
                        ms.poll.deregister(&listener).expect("wer");
                        ms.poll.register(&stream, token, ready_w, edge).expect("3y5");
                        todos[index] = Some(Todo::PassiveConnecting { stream, channel_id });
                    }
                    Some(Todo::ActiveConnecting { mut stream }) => {
                        let todo = if Self::test_stream_connectivity(&mut stream) {
                            ms.poll.reregister(&stream, token, ready_r, edge).expect("52");
                            let endpoint = Endpoint::from_fresh_stream(stream);
                            Todo::ActiveRecving { endpoint }
                        } else {
                            ms.poll.deregister(&stream).expect("wt");
                            std::thread::sleep(Duration::from_millis(backoff_millis));
                            backoff_millis = ((backoff_millis as f32) * 1.2) as u64 + 3;
                            let stream = TcpStream::connect(&binding.addr).unwrap();
                            ms.poll.register(&stream, token, ready_w, edge).expect("PAC 3");
                            Todo::ActiveConnecting { stream }
                        };
                        todos[index] = Some(todo);
                    }
                    Some(Todo::PassiveConnecting { mut stream, channel_id }) => {
                        if !Self::test_stream_connectivity(&mut stream) {
                            return Err(ConnectionShutdown(binding.addr));
                        }
                        ms.poll.reregister(&stream, token, ready_r, edge).expect("55");
                        let polarity = binding.polarity;
                        let info = EndpointInfo { polarity, channel_id };
                        let msg = Msg::SetupMsg(SetupMsg::ChannelSetup { info });
                        let mut endpoint = Endpoint::from_fresh_stream(stream);
                        endpoint.send(msg).map_err(|e| EndpointErr(Port(index), e))?;
                        let endpoint_ext = EndpointExt { endpoint, info };
                        endpoint_exts.occupy_reserved(index, endpoint_ext);
                        num_todos_remaining -= 1;
                    }
                    Some(Todo::ActiveRecving { mut endpoint }) => {
                        let ekey = Port(index);
                        'recv_loop: while let Some(msg) =
                            endpoint.recv().map_err(|e| EndpointErr(ekey, e))?
                        {
                            if let Msg::SetupMsg(SetupMsg::ChannelSetup { info }) = msg {
                                if info.polarity == binding.polarity {
                                    return Err(PortKindMismatch(ekey, binding.addr));
                                }
                                let channel_id = info.channel_id;
                                let info = EndpointInfo { polarity: binding.polarity, channel_id };
                                ms.polled_undrained.insert(ekey);
                                let endpoint_ext = EndpointExt { endpoint, info };
                                endpoint_exts.occupy_reserved(index, endpoint_ext);
                                num_todos_remaining -= 1;
                                break 'recv_loop;
                            } else {
                                ms.delayed.push(ReceivedMsg { recipient: ekey, msg });
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(None, endpoint_exts.iter_reserved().next());
        drop(todos);

        ///////////////////////////////////////////////////////
        // 1. construct `family', i.e. perform the sink tree setup procedure
        use {Msg::SetupMsg as S, SetupMsg::*};
        let mut messenger = (&mut ms, &mut endpoint_exts);
        impl Messengerlike for (&mut MessengerState, &mut VecStorage<EndpointExt>) {
            fn get_state_mut(&mut self) -> &mut MessengerState {
                self.0
            }
            fn get_endpoint_mut(&mut self, ekey: Key) -> &mut Endpoint {
                &mut self
                    .1
                    .get_occupied_mut(ekey.to_raw() as usize)
                    .expect("OUT OF BOUNDS")
                    .endpoint
            }
        }

        // 1. broadcast my ID as the first echo. await reply from all in net_keylist
        let neighbors = (0..self.bindings.len()).map(Port);
        let echo = S(LeaderEcho { maybe_leader: controller_id });
        let mut awaiting = IndexSet::<Port>::with_capacity(neighbors.len());
        for n in neighbors.clone() {
            messenger.send(n, echo.clone()).map_err(|e| EndpointErr(n, e))?;
            awaiting.insert(n);
        }

        // 2. Receive incoming replies. whenever a higher-id echo arrives,
        //    adopt it as leader, sender as parent, and reset the await set.
        let mut parent: Option<Port> = None;
        let mut my_leader = controller_id;
        messenger.undelay_all();
        'echo_loop: while !awaiting.is_empty() || parent.is_some() {
            let ReceivedMsg { recipient, msg } = messenger.recv_until(deadline)?.ok_or(Timeout)?;
            match msg {
                S(LeaderAnnounce { leader }) => {
                    // someone else completed the echo and became leader first!
                    // the sender is my parent
                    parent = Some(recipient);
                    my_leader = leader;
                    awaiting.clear();
                    break 'echo_loop;
                }
                S(LeaderEcho { maybe_leader }) => {
                    use Ordering::*;
                    match maybe_leader.cmp(&my_leader) {
                        Less => { /* ignore */ }
                        Equal => {
                            awaiting.remove(&recipient);
                            if awaiting.is_empty() {
                                if let Some(p) = parent {
                                    // return the echo to my parent
                                    messenger
                                        .send(p, S(LeaderEcho { maybe_leader }))
                                        .map_err(|e| EndpointErr(p, e))?;
                                } else {
                                    // DECIDE!
                                    break 'echo_loop;
                                }
                            }
                        }
                        Greater => {
                            // join new echo
                            parent = Some(recipient);
                            my_leader = maybe_leader;
                            let echo = S(LeaderEcho { maybe_leader: my_leader });
                            awaiting.clear();
                            if neighbors.len() == 1 {
                                // immediately reply to parent
                                messenger
                                    .send(recipient, echo.clone())
                                    .map_err(|e| EndpointErr(recipient, e))?;
                            } else {
                                for n in neighbors.clone() {
                                    if n != recipient {
                                        messenger
                                            .send(n, echo.clone())
                                            .map_err(|e| EndpointErr(n, e))?;
                                        awaiting.insert(n);
                                    }
                                }
                            }
                        }
                    }
                }
                msg => messenger.delay(ReceivedMsg { recipient, msg }),
            }
        }
        match parent {
            None => assert_eq!(
                my_leader, controller_id,
                "I've got no parent, but I consider {:?} the leader?",
                my_leader
            ),
            Some(parent) => assert_ne!(
                my_leader, controller_id,
                "I have {:?} as parent, but I consider myself ({:?}) the leader?",
                parent, controller_id
            ),
        }

        // 3. broadcast leader announcement (except to parent: confirm they are your parent)
        //    in this loop, every node sends 1 message to each neighbor
        let msg_for_non_parents = S(LeaderAnnounce { leader: my_leader });
        for n in neighbors.clone() {
            let msg =
                if Some(n) == parent { S(YouAreMyParent) } else { msg_for_non_parents.clone() };
            messenger.send(n, msg).map_err(|e| EndpointErr(n, e))?;
        }

        // await 1 message from all non-parents
        for n in neighbors.clone() {
            if Some(n) != parent {
                awaiting.insert(n);
            }
        }
        let mut children = HashSet::default();
        messenger.undelay_all();
        while !awaiting.is_empty() {
            let ReceivedMsg { recipient, msg } = messenger.recv_until(deadline)?.ok_or(Timeout)?;
            let recipient = recipient;
            match msg {
                S(YouAreMyParent) => {
                    assert!(awaiting.remove(&recipient));
                    children.insert(recipient);
                }
                S(SetupMsg::LeaderAnnounce { leader }) => {
                    assert!(awaiting.remove(&recipient));
                    assert!(leader == my_leader);
                    assert!(Some(recipient) != parent);
                    // they wouldn't send me this if they considered me their parent
                }
                _ => messenger.delay(ReceivedMsg { recipient, msg }),
            }
        }
        let family = Family { parent, children };

        // done!
        Ok(Connected {
            components: Default::default(),
            controller_id,
            channel_index_stream,
            endpoint_exts,
            native_ports,
            family,
            ephemeral: Default::default(),
        })
    }
    /////////
    pub fn connect_using_id(
        &mut self,
        controller_id: ControllerId,
        timeout: Option<Duration>,
    ) -> Result<Connected, ConnectErr> {
        // 1. try and create a connection from these bindings with self immutable.
        let connected = self.new_connected(controller_id, timeout)?;
        // 2. success! drain self and return
        self.bindings.clear();
        Ok(connected)
    }
    pub fn connect(&mut self, timeout: Option<Duration>) -> Result<Connected, ConnectErr> {
        self.connect_using_id(Self::random_controller_id(), timeout)
    }
}

#[derive(Debug)]
pub struct Connected {
    native_ports: HashSet<Port>,
    controller_id: ControllerId,
    channel_index_stream: ChannelIndexStream,
    endpoint_exts: VecStorage<EndpointExt>,
    components: VecStorage<Component>,
    family: Family,
    ephemeral: Ephemeral,
}
#[derive(Debug, Default)]
struct Ephemeral {
    // invariant: between rounds this is cleared
    machines: Vec<Machine>,
    bit_matrix: BitMatrix,
    assignment_to_bit_property: HashMap<(ChannelId, bool), usize>,
    usize_buf: Vec<usize>,
}
impl Ephemeral {
    fn clear(&mut self) {
        self.bit_matrix = Default::default();
        self.usize_buf.clear();
        self.machines.clear();
        self.assignment_to_bit_property.clear();
    }
}
#[derive(Debug)]
struct Machine {
    component_index: usize,
    state: ProtocolS,
}
struct MonoCtx<'a> {
    another_pass: &'a mut bool,
}
impl MonoContext for MonoCtx<'_> {
    type D = ProtocolD;
    type S = ProtocolS;

    fn new_component(&mut self, moved_keys: HashSet<Key>, init_state: Self::S) {
        todo!()
    }
    fn new_channel(&mut self) -> [Key; 2] {
        todo!()
    }
    fn new_random(&mut self) -> u64 {
        todo!()
    }
}
impl Connected {
    pub fn new_component(
        &mut self,
        protocol: &Arc<ProtocolD>,
        identifier: &Arc<[u8]>,
        moved_port_list: &[Port],
    ) -> Result<(), MainComponentErr> {
        //////////////////////////////////////////
        // 1. try and create a new component (without mutating self)
        use MainComponentErr::*;
        let moved_port_set = {
            let mut set: HashSet<Port> = Default::default();
            for &port in moved_port_list.iter() {
                if !self.native_ports.contains(&port) {
                    return Err(CannotMovePort(port));
                }
                if !set.insert(port) {
                    return Err(DuplicateMovedPort(port));
                }
            }
            set
        };
        // moved_port_set is disjoint to native_ports
        let expected_polarities = protocol.component_polarities(identifier)?;
        if moved_port_list.len() != expected_polarities.len() {
            return Err(WrongNumberOfParamaters { expected: expected_polarities.len() });
        }
        // correct polarity list
        for (param_index, (&port, &expected_polarity)) in
            moved_port_list.iter().zip(expected_polarities.iter()).enumerate()
        {
            let polarity =
                self.endpoint_exts.get_occupied(port.0).ok_or(UnknownPort(port))?.info.polarity;
            if polarity != expected_polarity {
                return Err(WrongPortPolarity { param_index, port });
            }
        }
        let state = Some(protocol.new_main_component(identifier, &moved_port_list));
        let component = Component {
            port_set: moved_port_set,
            protocol: protocol.clone(),
            identifier: identifier.clone(),
            state,
        };
        //////////////////////////////
        // success! mutate self and return Ok
        self.native_ports.retain(|e| !component.port_set.contains(e));
        self.components.new_occupied(component);
        Ok(())
    }
    pub fn new_channel(&mut self) -> (OutPort, InPort) {
        assert!(self.endpoint_exts.len() <= std::u32::MAX as usize - 2);
        let channel_id = ChannelId {
            controller_id: self.controller_id,
            channel_index: self.channel_index_stream.next(),
        };
        let [e0, e1] = Endpoint::new_memory_pair();
        let kp = self.endpoint_exts.new_occupied(EndpointExt {
            info: EndpointInfo { channel_id, polarity: Putter },
            endpoint: e0,
        });
        let kg = self.endpoint_exts.new_occupied(EndpointExt {
            info: EndpointInfo { channel_id, polarity: Getter },
            endpoint: e1,
        });
        (OutPort(Port(kp)), InPort(Port(kg)))
    }
    pub fn sync_set(&mut self, _inbuf: &mut [u8], _ops: &mut [PortOpRs]) -> Result<(), ()> {
        // For every component, take its state and make a singleton machine
        for (component_index, component) in self.components.iter_mut().enumerate() {
            let state = component.state.take().unwrap();
            let machine = Machine { component_index, state };
            self.ephemeral.machines.push(machine);
        }

        // Grow property matrix. has |machines| entities and {to_run => 0, to_remove => 1} properties
        const PROP_TO_RUN: usize = 0;
        const PROP_TO_REMOVE: usize = 1;
        self.ephemeral
            .bit_matrix
            .grow_to(Pair { property: 2, entity: self.ephemeral.machines.len() as u32 });
        // Set to_run property for all existing machines
        self.ephemeral.bit_matrix.batch_mut(move |p| p[PROP_TO_RUN] = TRUE_CHUNK);

        /////////////
        // perform mono runs, adding and removing TO_RUN property bits bits, and adding PROP_TO_REMOVE property bits
        let mut usize_buf = vec![];
        let mut another_pass = true;
        while another_pass {
            another_pass = false;
            let machine_index_iter = self
                .ephemeral
                .bit_matrix
                .iter_entities_where(&mut usize_buf, move |p| p[PROP_TO_RUN]);
            for machine_index in machine_index_iter {
                let machine = &mut self.ephemeral.machines[machine_index as usize];
                let component = self.components.get_occupied(machine.component_index).unwrap();
                let mut ctx = MonoCtx { another_pass: &mut another_pass };
                // TODO ctx doesn't work. it may callback to create new machines (setting their TO_RUN and another_pass=true)
                match machine.state.pre_sync_run(&mut ctx, &component.protocol) {
                    MonoBlocker::Inconsistent => todo!(), // make entire state inconsistent!
                    MonoBlocker::ComponentExit => self
                        .ephemeral
                        .bit_matrix
                        .set(Pair { entity: machine_index, property: PROP_TO_REMOVE as u32 }),
                    MonoBlocker::SyncBlockStart => self
                        .ephemeral
                        .bit_matrix
                        .unset(Pair { entity: machine_index, property: PROP_TO_RUN as u32 }),
                }
            }
        }
        // no machines have property TO_RUN

        // from back to front, swap_remove all machines with PROP_TO_REMOVE
        let machine_index_iter = self
            .ephemeral
            .bit_matrix
            .iter_entities_where_rev(&mut usize_buf, move |p| p[PROP_TO_REMOVE]);
        for machine_index in machine_index_iter {
            let machine = self.ephemeral.machines.swap_remove(machine_index as usize);
            drop(machine);
        }

        // replace old matrix full of bogus data with a new (fresh) one for the set of machines
        // henceforth, machines(entities) and properties won't shrink or move.
        self.ephemeral.bit_matrix =
            BitMatrix::new(Pair { entity: self.ephemeral.machines.len() as u32 * 2, property: 8 });

        // !!! TODO poly run until solution is found

        ////////////////////
        let solution_assignments: Vec<(ChannelId, bool)> = vec![];
        // solution has been found. time to find a

        // logically destructure self so we can read and write to different fields interleaved...
        let Self {
            components,
            ephemeral: Ephemeral { bit_matrix, assignment_to_bit_property, usize_buf, machines },
            ..
        } = self;

        // !!!!!!! TODO MORE HERE

        let machine_index_iter = bit_matrix.iter_entities_where(usize_buf, move |p| {
            solution_assignments.iter().fold(TRUE_CHUNK, |chunk, assignment| {
                let &bit_property = assignment_to_bit_property.get(assignment).unwrap();
                chunk & p[bit_property]
            })
        });
        for machine_index in machine_index_iter {
            let machine = &machines[machine_index as usize];
            let component = &mut components.get_occupied_mut(machine.component_index).unwrap();
            let was = component.state.replace(machine.state.clone());
            assert!(was.is_none()); // 2+ machines matched the solution for this component!
            println!("visiting machine at index {:?}", machine_index);
        }
        for component in self.components.iter() {
            assert!(component.state.is_some()); // 0 machines matched the solution for this component!
        }
        self.ephemeral.clear();
        println!("B {:#?}", self);
        Ok(())
    }
    pub fn sync_subsets(
        &mut self,
        _inbuf: &mut [u8],
        _ops: &mut [PortOpRs],
        bit_subsets: &[&[usize]],
    ) -> Result<usize, ()> {
        for (batch_index, bit_subset) in bit_subsets.iter().enumerate() {
            println!("batch_index {:?}", batch_index);
            let chunk_iter = bit_subset.iter().copied();
            for index in BitChunkIter::new(chunk_iter) {
                println!("  index {:?}", index);
            }
        }
        Ok(0)
    }
}

macro_rules! bitslice {
    ($( $num:expr  ),*) => {{
        &[0 $( | (1usize << $num)  )*]
    }};
}

#[test]
fn api_new_test() {
    let mut c = Connecting::default();
    let net_out: OutPort = c.bind(Coupling::Active, "127.0.0.1:8000".parse().unwrap());
    let net_in: InPort = c.bind(Coupling::Active, "127.0.0.1:8001".parse().unwrap());
    let proto_0 = Arc::new(ProtocolD::parse(b"").unwrap());
    let mut c = c.connect(None).unwrap();
    let (mem_out, mem_in) = c.new_channel();
    let mut inbuf = [0u8; 64];
    let identifier: Arc<[u8]> = b"sync".to_vec().into();
    c.new_component(&proto_0, &identifier, &[net_in.into(), mem_out.into()]).unwrap();
    let mut ops = [
        PortOpRs::In { msg_range: None, port: &mem_in },
        PortOpRs::Out { msg: b"hey", port: &net_out, optional: false },
        PortOpRs::Out { msg: b"hi?", port: &net_out, optional: true },
        PortOpRs::Out { msg: b"yo!", port: &net_out, optional: false },
    ];
    c.sync_set(&mut inbuf, &mut ops).unwrap();
    c.sync_subsets(&mut inbuf, &mut ops, &[bitslice! {0,1,2}]).unwrap();
}

#[repr(C)]
pub struct PortOp {
    msgptr: *mut u8, // read if OUT, field written if IN, will point into buf
    msglen: usize,   // read if OUT, written if IN, won't exceed buf
    port: Port,
    optional: bool, // no meaning if
}

pub enum PortOpRs<'a> {
    In { msg_range: Option<Range<usize>>, port: &'a InPort },
    Out { msg: &'a [u8], port: &'a OutPort, optional: bool },
}

unsafe fn c_sync_set(
    connected: &mut Connected,
    inbuflen: usize,
    inbufptr: *mut u8,
    opslen: usize,
    opsptr: *mut PortOp,
) -> i32 {
    let buf = as_mut_slice(inbuflen, inbufptr);
    let ops = as_mut_slice(opslen, opsptr);
    let (subset_index, wrote) = sync_inner(connected, buf);
    assert_eq!(0, subset_index);
    for op in ops {
        if let Some(range) = wrote.get(&op.port) {
            op.msgptr = inbufptr.add(range.start);
            op.msglen = range.end - range.start;
        }
    }
    0
}

unsafe fn c_sync_subset(
    connected: &mut Connected,
    inbuflen: usize,
    inbufptr: *mut u8,
    opslen: usize,
    opsptr: *mut PortOp,
    subsetslen: usize,
    subsetsptr: *const *const usize,
) -> i32 {
    let buf: &mut [u8] = as_mut_slice(inbuflen, inbufptr);
    let ops: &mut [PortOp] = as_mut_slice(opslen, opsptr);
    let subsets: &[*const usize] = as_const_slice(subsetslen, subsetsptr);
    let subsetlen = usizes_for_bits(opslen);
    // don't yet know subsetptr; which subset fires unknown!

    let (subset_index, wrote) = sync_inner(connected, buf);
    let subsetptr: *const usize = subsets[subset_index];
    let subset: &[usize] = as_const_slice(subsetlen, subsetptr);

    for index in BitChunkIter::new(subset.iter().copied()) {
        let op = &mut ops[index as usize];
        if let Some(range) = wrote.get(&op.port) {
            op.msgptr = inbufptr.add(range.start);
            op.msglen = range.end - range.start;
        }
    }
    subset_index as i32
}

// dummy fn for the actual synchronous round
fn sync_inner<'c, 'b>(
    _connected: &'c mut Connected,
    _buf: &'b mut [u8],
) -> (usize, &'b HashMap<Port, Range<usize>>) {
    todo!()
}

unsafe fn as_mut_slice<'a, T>(len: usize, ptr: *mut T) -> &'a mut [T] {
    std::slice::from_raw_parts_mut(ptr, len)
}
unsafe fn as_const_slice<'a, T>(len: usize, ptr: *const T) -> &'a [T] {
    std::slice::from_raw_parts(ptr, len)
}

#[test]
fn api_connecting() {
    let addrs: [SocketAddr; 3] = [
        "127.0.0.1:8888".parse().unwrap(),
        "127.0.0.1:8889".parse().unwrap(),
        "127.0.0.1:8890".parse().unwrap(),
    ];

    lazy_static::lazy_static! {
        static ref PROTOCOL: Arc<ProtocolD> = {
            static PDL: &[u8] = b"
            primitive sync(in i, out o) {
                while(true) synchronous {
                    put(o, get(i));
                }
            }
            ";
            Arc::new(ProtocolD::parse(PDL).unwrap())
        };
    }

    const TIMEOUT: Option<Duration> = Some(Duration::from_secs(1));
    let handles = vec![
        std::thread::spawn(move || {
            let mut c = Connecting::default();
            let p_in: InPort = c.bind(Coupling::Passive, addrs[0]);
            let p_out: OutPort = c.bind(Coupling::Active, addrs[1]);
            let mut c = c.connect(TIMEOUT).unwrap();
            println!("c {:#?}", &c);

            let identifier = b"sync".to_vec().into();
            c.new_component(&PROTOCOL, &identifier, &[p_in.into(), p_out.into()]).unwrap();
            println!("c {:#?}", &c);

            let mut inbuf = vec![];
            let mut port_ops = [];
            c.sync_set(&mut inbuf, &mut port_ops).unwrap();
        }),
        std::thread::spawn(move || {
            let mut connecting = Connecting::default();
            let _a: OutPort = connecting.bind(Coupling::Active, addrs[0]);
            let _b: InPort = connecting.bind(Coupling::Passive, addrs[1]);
            let _c: InPort = connecting.bind(Coupling::Active, addrs[2]);
            let _connected = connecting.connect(TIMEOUT).unwrap();
        }),
        std::thread::spawn(move || {
            let mut connecting = Connecting::default();
            let _a: OutPort = connecting.bind(Coupling::Passive, addrs[2]);
            let _connected = connecting.connect(TIMEOUT).unwrap();
        }),
    ];
    for h in handles {
        h.join().unwrap();
    }
}
