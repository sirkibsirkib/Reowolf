use super::vec_storage::VecStorage;
use crate::common::*;
use crate::runtime::endpoint::EndpointExt;
use crate::runtime::endpoint::EndpointInfo;
use crate::runtime::endpoint::{Endpoint, Msg, SetupMsg};
use crate::runtime::MessengerState;
use crate::runtime::Messengerlike;
use crate::runtime::ReceivedMsg;

use std::net::SocketAddr;
use std::sync::Arc;

pub enum Coupling {
    Active,
    Passive,
}

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

#[derive(Default)]
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
        protocol: &Arc<Protocol>,
        timeout: Option<Duration>,
    ) -> Result<Connected, ()> {
        // TEMP: helper functions until Key is unified with Port
        #[inline]
        fn key2port(ekey: Key) -> Port {
            Port(ekey.to_raw() as usize)
        }
        #[inline]
        fn port2key(port: Port) -> Key {
            Key::from_raw(port.0)
        }

        // 1. bindings correspond with ports 0..bindings.len(). For each:
        //    - reserve a slot in endpoint_exts.
        //    - store the port in `native_ports' set.
        let mut endpoint_exts = VecStorage::<EndpointExt>::with_reserved_range(self.bindings.len());
        let native_ports = (0..self.bindings.len()).map(Port).collect();

        // 2. create MessengerState structure for polling channels
        let edge = PollOpt::edge();
        let [ready_r, ready_w] = [Ready::readable(), Ready::writable()];
        let mut ms = MessengerState {
            poll: Poll::new().map_err(drop)?,
            events: Events::with_capacity(self.bindings.len()),
            delayed: vec![],
            undelayed: vec![],
            polled_undrained: Default::default(),
        };

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
                        let listener = TcpListener::bind(&binding.addr).map_err(drop)?;
                        ms.poll.register(&listener, Token(index), ready_r, edge).unwrap(); // registration unique
                        Todo::PassiveAccepting { listener, channel_id }
                    }
                    Coupling::Active => {
                        let stream = TcpStream::connect(&binding.addr).map_err(drop)?;
                        ms.poll.register(&stream, Token(index), ready_w, edge).unwrap(); // registration unique
                        Todo::ActiveConnecting { stream }
                    }
                }))
            })
            .collect::<Result<Vec<Option<Todo>>, ()>>()?;
        let mut num_todos_remaining = todos.len();

        // 4. handle incoming events until all TODOs are completed OR we timeout
        let deadline = timeout.map(|t| Instant::now() + t);
        let mut polled_undrained_later = IndexSet::<_>::default();
        let mut backoff_millis = 10;
        while num_todos_remaining > 0 {
            ms.poll_events_until(deadline).map_err(drop)?;
            for event in ms.events.iter() {
                let token = event.token();
                let index = token.0;
                let binding = &self.bindings[index];
                match todos[index].take() {
                    None => {
                        polled_undrained_later.insert(index);
                    }
                    Some(Todo::PassiveAccepting { listener, channel_id }) => {
                        let (stream, _peer_addr) = listener.accept().map_err(drop)?;
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
                            return Err(());
                        }
                        ms.poll.reregister(&stream, token, ready_r, edge).expect("55");
                        let polarity = binding.polarity;
                        let info = EndpointInfo { polarity, channel_id };
                        let msg = Msg::SetupMsg(SetupMsg::ChannelSetup { info });
                        let mut endpoint = Endpoint::from_fresh_stream(stream);
                        endpoint.send(msg).map_err(drop)?;
                        let endpoint_ext = EndpointExt { endpoint, info };
                        endpoint_exts.occupy_reserved(index, endpoint_ext);
                        num_todos_remaining -= 1;
                    }
                    Some(Todo::ActiveRecving { mut endpoint }) => {
                        // log!(logger, "{:03?} start ActiveRecving...", major);
                        // assert!(event.readiness().is_readable());
                        let ekey = Key::from_raw(index);
                        'recv_loop: while let Some(msg) = endpoint.recv().map_err(drop)? {
                            if let Msg::SetupMsg(SetupMsg::ChannelSetup { info }) = msg {
                                if info.polarity == binding.polarity {
                                    return Err(());
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
            messenger.send(port2key(n), echo.clone()).map_err(drop)?;
            awaiting.insert(n);
        }

        // 2. Receive incoming replies. whenever a higher-id echo arrives,
        //    adopt it as leader, sender as parent, and reset the await set.
        let mut parent: Option<Port> = None;
        let mut my_leader = controller_id;
        messenger.undelay_all();
        'echo_loop: while !awaiting.is_empty() || parent.is_some() {
            let ReceivedMsg { recipient, msg } =
                messenger.recv_until(deadline).map_err(drop)?.ok_or(())?;
            let recipient = key2port(recipient);
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
                                        .send(port2key(p), S(LeaderEcho { maybe_leader }))
                                        .map_err(drop)?;
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
                                messenger.send(port2key(recipient), echo.clone()).map_err(drop)?;
                            } else {
                                for n in neighbors.clone() {
                                    if n != recipient {
                                        messenger.send(port2key(n), echo.clone()).map_err(drop)?;
                                        awaiting.insert(n);
                                    }
                                }
                            }
                        }
                    }
                }
                msg => messenger.delay(ReceivedMsg { recipient: port2key(recipient), msg }),
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
            messenger.send(port2key(n), msg).map_err(drop)?;
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
            let ReceivedMsg { recipient, msg } =
                messenger.recv_until(deadline).map_err(drop)?.ok_or(())?;
            let recipient = key2port(recipient);
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
                _ => messenger.delay(ReceivedMsg { recipient: port2key(recipient), msg }),
            }
        }
        let family = Family { parent, children };

        // 1. done! return
        Ok(Connected {
            controller_id,
            channel_index_stream,
            protocol: protocol.clone(),
            endpoint_exts,
            native_ports,
            family,
        })
    }
    /////////
    pub fn connect_using_id(
        &mut self,
        controller_id: ControllerId,
        protocol: &Arc<Protocol>,
        timeout: Option<Duration>,
    ) -> Result<Connected, ()> {
        // 1. try and create a connection from these bindings with self immutable.
        let connected = self.new_connected(controller_id, protocol, timeout)?;
        // 2. success! drain self and return
        self.bindings.clear();
        Ok(connected)
    }
    pub fn connect(
        &mut self,
        protocol: &Arc<Protocol>,
        timeout: Option<Duration>,
    ) -> Result<Connected, ()> {
        self.connect_using_id(Self::random_controller_id(), protocol, timeout)
    }
}
pub struct Protocol;
impl Protocol {
    pub fn parse(_pdl_text: &[u8]) -> Result<Self, ()> {
        Ok(Protocol)
    }
}
// struct ComponentExt {
//     protocol: Arc<Protocol>,
//     ports: HashSet<Port>,
//     name: Vec<u8>,
// }
pub struct Connected {
    native_ports: HashSet<Port>,
    controller_id: ControllerId,
    channel_index_stream: ChannelIndexStream,
    endpoint_exts: VecStorage<EndpointExt>,
    protocol: Arc<Protocol>,
    family: Family,
    // components: Vec<ComponentExt>,
}
impl Connected {
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
    pub fn new_component(&mut self, _name: Vec<u8>, moved_ports: &[Port]) -> Result<(), ()> {
        let moved_ports = moved_ports.iter().copied().collect();
        if !self.native_ports.is_superset(&moved_ports) {
            return Err(());
        }
        self.native_ports.retain(|e| !moved_ports.contains(e));
        // self.components.push(ComponentExt { ports: moved_ports, protocol: protocol.clone(), name });
        todo!()
    }
    pub fn sync_set(&mut self, _inbuf: &mut [u8], _ops: &mut [PortOpRs]) -> Result<(), ()> {
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
    let proto_0 = Arc::new(Protocol::parse(b"").unwrap());
    let mut c = c.connect(&proto_0, None).unwrap();
    let (mem_out, mem_in) = c.new_channel();
    let mut inbuf = [0u8; 64];
    c.new_component(b"sync".to_vec(), &[net_in.into(), mem_out.into()]).unwrap();
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

use super::bits::{usizes_for_bits, BitChunkIter};
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
