#[cfg(feature = "ffi")]
pub mod ffi;

mod actors;
pub(crate) mod communication;
pub(crate) mod connector;
pub(crate) mod endpoint;
pub mod errors;
// pub mod experimental;
mod serde;
pub(crate) mod setup;

pub(crate) type ProtocolD = crate::protocol::ProtocolDescriptionImpl;
pub(crate) type ProtocolS = crate::protocol::ComponentStateImpl;

use crate::common::*;
use actors::*;
use endpoint::*;
use errors::*;

#[derive(Debug, PartialEq)]
pub(crate) enum CommonSatResult {
    FormerNotLatter,
    LatterNotFormer,
    Equivalent,
    New(Predicate),
    Nonexistant,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub(crate) struct Predicate {
    pub assigned: BTreeMap<ChannelId, bool>,
}

#[derive(Debug, Default)]
struct SyncBatch {
    puts: HashMap<Port, Payload>,
    gets: HashSet<Port>,
}

#[derive(Debug)]
pub enum Connector {
    Unconfigured(Unconfigured),
    Configured(Configured),
    Connected(Connected), // TODO consider boxing. currently takes up a lot of stack real estate
}
#[derive(Debug)]
pub struct Unconfigured {
    pub controller_id: ControllerId,
}
#[derive(Debug)]
pub struct Configured {
    controller_id: ControllerId,
    polarities: Vec<Polarity>,
    bindings: HashMap<usize, PortBinding>,
    protocol_description: Arc<ProtocolD>,
    main_component: Vec<u8>,
    logger: String,
}
#[derive(Debug)]
pub struct Connected {
    native_interface: Vec<(Port, Polarity)>,
    sync_batches: Vec<SyncBatch>,
    controller: Controller,
}

#[derive(Debug, Copy, Clone)]
pub enum PortBinding {
    Native,
    Active(SocketAddr),
    Passive(SocketAddr),
}

#[derive(Debug)]
struct Arena<T> {
    storage: Vec<T>,
}

#[derive(Debug)]
struct ReceivedMsg {
    recipient: Port,
    msg: Msg,
}

#[derive(Debug)]
struct MessengerState {
    poll: Poll,
    events: Events,
    delayed: Vec<ReceivedMsg>,
    undelayed: Vec<ReceivedMsg>,
    polled_undrained: IndexSet<Port>,
}
#[derive(Debug)]
struct ChannelIdStream {
    controller_id: ControllerId,
    next_channel_index: ChannelIndex,
}

#[derive(Debug)]
struct Controller {
    protocol_description: Arc<ProtocolD>,
    inner: ControllerInner,
    ephemeral: ControllerEphemeral,
    unrecoverable_error: Option<SyncErr>, // prevents future calls to Sync
}
#[derive(Debug)]
struct ControllerInner {
    round_index: usize,
    channel_id_stream: ChannelIdStream,
    endpoint_exts: Arena<EndpointExt>,
    messenger_state: MessengerState,
    mono_n: MonoN,       // state at next round start
    mono_ps: Vec<MonoP>, // state at next round start
    family: ControllerFamily,
    logger: String,
}

/// This structure has its state entirely reset between synchronous rounds
#[derive(Debug, Default)]
struct ControllerEphemeral {
    solution_storage: SolutionStorage,
    poly_n: Option<PolyN>,
    poly_ps: Vec<PolyP>,
    mono_ps: Vec<MonoP>,
    port_to_holder: HashMap<Port, PolyId>,
}

#[derive(Debug)]
struct ControllerFamily {
    parent_port: Option<Port>,
    children_ports: Vec<Port>,
}

#[derive(Debug)]
pub(crate) enum SyncRunResult {
    BlockingForRecv,
    AllBranchesComplete,
    NoBranches,
}

// Used to identify poly actors
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum PolyId {
    N,
    P { index: usize },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum SubtreeId {
    PolyN,
    PolyP { index: usize },
    ChildController { port: Port },
}

pub(crate) struct MonoPContext<'a> {
    inner: &'a mut ControllerInner,
    ports: &'a mut HashSet<Port>,
    mono_ps: &'a mut Vec<MonoP>,
}
pub(crate) struct PolyPContext<'a> {
    my_subtree_id: SubtreeId,
    inner: &'a mut ControllerInner,
    solution_storage: &'a mut SolutionStorage,
}
impl PolyPContext<'_> {
    #[inline(always)]
    fn reborrow<'a>(&'a mut self) -> PolyPContext<'a> {
        let Self { solution_storage, my_subtree_id, inner } = self;
        PolyPContext { solution_storage, my_subtree_id: *my_subtree_id, inner }
    }
}
struct BranchPContext<'m, 'r> {
    m_ctx: PolyPContext<'m>,
    ports: &'r HashSet<Port>,
    predicate: &'r Predicate,
    inbox: &'r HashMap<Port, Payload>,
}

#[derive(Default)]
pub(crate) struct SolutionStorage {
    old_local: HashSet<Predicate>,
    new_local: HashSet<Predicate>,
    // this pair acts as SubtreeId -> HashSet<Predicate> which is friendlier to iteration
    subtree_solutions: Vec<HashSet<Predicate>>,
    subtree_id_to_index: HashMap<SubtreeId, usize>,
}

trait Messengerlike {
    fn get_state_mut(&mut self) -> &mut MessengerState;
    fn get_endpoint_mut(&mut self, eport: Port) -> &mut Endpoint;

    fn delay(&mut self, received: ReceivedMsg) {
        self.get_state_mut().delayed.push(received);
    }
    fn undelay_all(&mut self) {
        let MessengerState { delayed, undelayed, .. } = self.get_state_mut();
        undelayed.extend(delayed.drain(..))
    }

    fn send(&mut self, to: Port, msg: Msg) -> Result<(), EndpointErr> {
        self.get_endpoint_mut(to).send(msg)
    }

    // attempt to receive a message from one of the endpoints before the deadline
    fn recv(&mut self, deadline: Instant) -> Result<Option<ReceivedMsg>, MessengerRecvErr> {
        // try get something buffered
        if let Some(x) = self.get_state_mut().undelayed.pop() {
            return Ok(Some(x));
        }

        loop {
            // polled_undrained may not be empty
            while let Some(eport) = self.get_state_mut().polled_undrained.pop() {
                if let Some(msg) = self
                    .get_endpoint_mut(eport)
                    .recv()
                    .map_err(|e| MessengerRecvErr::EndpointErr(eport, e))?
                {
                    // this endpoint MAY still have messages! check again in future
                    self.get_state_mut().polled_undrained.insert(eport);
                    return Ok(Some(ReceivedMsg { recipient: eport, msg }));
                }
            }

            let state = self.get_state_mut();
            match state.poll_events(deadline) {
                Ok(()) => {
                    for e in state.events.iter() {
                        state.polled_undrained.insert(Port::from_token(e.token()));
                    }
                }
                Err(PollDeadlineErr::PollingFailed) => return Err(MessengerRecvErr::PollingFailed),
                Err(PollDeadlineErr::Timeout) => return Ok(None),
            }
        }
    }
    fn recv_blocking(&mut self) -> Result<ReceivedMsg, MessengerRecvErr> {
        // try get something buffered
        if let Some(x) = self.get_state_mut().undelayed.pop() {
            return Ok(x);
        }

        loop {
            // polled_undrained may not be empty
            while let Some(eport) = self.get_state_mut().polled_undrained.pop() {
                if let Some(msg) = self
                    .get_endpoint_mut(eport)
                    .recv()
                    .map_err(|e| MessengerRecvErr::EndpointErr(eport, e))?
                {
                    // this endpoint MAY still have messages! check again in future
                    self.get_state_mut().polled_undrained.insert(eport);
                    return Ok(ReceivedMsg { recipient: eport, msg });
                }
            }

            let state = self.get_state_mut();

            state
                .poll
                .poll(&mut state.events, None)
                .map_err(|_| MessengerRecvErr::PollingFailed)?;
            for e in state.events.iter() {
                state.polled_undrained.insert(Port::from_token(e.token()));
            }
        }
    }
}

/////////////////////////////////
impl Debug for SolutionStorage {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.pad("Solutions: [")?;
        for (subtree_id, &index) in self.subtree_id_to_index.iter() {
            let sols = &self.subtree_solutions[index];
            f.write_fmt(format_args!("{:?}: {:?}, ", subtree_id, sols))?;
        }
        f.pad("]")
    }
}
impl From<EvalErr> for SyncErr {
    fn from(e: EvalErr) -> SyncErr {
        SyncErr::EvalErr(e)
    }
}
impl From<MessengerRecvErr> for SyncErr {
    fn from(e: MessengerRecvErr) -> SyncErr {
        SyncErr::MessengerRecvErr(e)
    }
}
impl From<MessengerRecvErr> for ConnectErr {
    fn from(e: MessengerRecvErr) -> ConnectErr {
        ConnectErr::MessengerRecvErr(e)
    }
}
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { storage: vec![] }
    }
}
impl<T> Arena<T> {
    pub fn alloc(&mut self, t: T) -> Port {
        self.storage.push(t);
        Port::from_raw(self.storage.len() - 1)
    }
    pub fn get(&self, key: Port) -> Option<&T> {
        self.storage.get(key.to_raw() as usize)
    }
    pub fn get_mut(&mut self, key: Port) -> Option<&mut T> {
        self.storage.get_mut(key.to_raw() as usize)
    }
    pub fn type_convert<X>(self, f: impl FnMut((Port, T)) -> X) -> Arena<X> {
        Arena { storage: self.keyspace().zip(self.storage.into_iter()).map(f).collect() }
    }
    pub fn iter(&self) -> impl Iterator<Item = (Port, &T)> {
        self.keyspace().zip(self.storage.iter())
    }
    pub fn len(&self) -> usize {
        self.storage.len()
    }
    pub fn keyspace(&self) -> impl Iterator<Item = Port> {
        (0..self.storage.len()).map(Port::from_raw)
    }
}

impl ChannelIdStream {
    fn new(controller_id: ControllerId) -> Self {
        Self { controller_id, next_channel_index: 0 }
    }
    fn next(&mut self) -> ChannelId {
        self.next_channel_index += 1;
        ChannelId { controller_id: self.controller_id, channel_index: self.next_channel_index - 1 }
    }
}

impl MessengerState {
    // does NOT guarantee that events is non-empty
    fn poll_events(&mut self, deadline: Instant) -> Result<(), PollDeadlineErr> {
        use PollDeadlineErr::*;
        self.events.clear();
        let poll_timeout = deadline.checked_duration_since(Instant::now()).ok_or(Timeout)?;
        self.poll.poll(&mut self.events, Some(poll_timeout)).map_err(|_| PollingFailed)?;
        Ok(())
    }
}
impl From<PollDeadlineErr> for ConnectErr {
    fn from(e: PollDeadlineErr) -> ConnectErr {
        match e {
            PollDeadlineErr::Timeout => ConnectErr::Timeout,
            PollDeadlineErr::PollingFailed => ConnectErr::PollingFailed,
        }
    }
}

impl std::ops::Not for Polarity {
    type Output = Self;
    fn not(self) -> Self::Output {
        use Polarity::*;
        match self {
            Putter => Getter,
            Getter => Putter,
        }
    }
}

impl Predicate {
    // returns true IFF self.unify would return Equivalent OR FormerNotLatter
    pub fn satisfies(&self, other: &Self) -> bool {
        let mut s_it = self.assigned.iter();
        let mut s = if let Some(s) = s_it.next() {
            s
        } else {
            return other.assigned.is_empty();
        };
        for (oid, ob) in other.assigned.iter() {
            while s.0 < oid {
                s = if let Some(s) = s_it.next() {
                    s
                } else {
                    return false;
                };
            }
            if s.0 > oid || s.1 != ob {
                return false;
            }
        }
        true
    }

    /// Given self and other, two predicates, return the most general Predicate possible, N
    /// such that n.satisfies(self) && n.satisfies(other).
    /// If none exists Nonexistant is returned.
    /// If the resulting predicate is equivlanet to self, other, or both,
    /// FormerNotLatter, LatterNotFormer and Equivalent are returned respectively.
    /// otherwise New(N) is returned.
    pub fn common_satisfier(&self, other: &Self) -> CommonSatResult {
        use CommonSatResult::*;
        // iterators over assignments of both predicates. Rely on SORTED ordering of BTreeMap's keys.
        let [mut s_it, mut o_it] = [self.assigned.iter(), other.assigned.iter()];
        let [mut s, mut o] = [s_it.next(), o_it.next()];
        // lists of assignments in self but not other and vice versa.
        let [mut s_not_o, mut o_not_s] = [vec![], vec![]];
        loop {
            match [s, o] {
                [None, None] => break,
                [None, Some(x)] => {
                    o_not_s.push(x);
                    o_not_s.extend(o_it);
                    break;
                }
                [Some(x), None] => {
                    s_not_o.push(x);
                    s_not_o.extend(s_it);
                    break;
                }
                [Some((sid, sb)), Some((oid, ob))] => {
                    if sid < oid {
                        // o is missing this element
                        s_not_o.push((sid, sb));
                        s = s_it.next();
                    } else if sid > oid {
                        // s is missing this element
                        o_not_s.push((oid, ob));
                        o = o_it.next();
                    } else if sb != ob {
                        assert_eq!(sid, oid);
                        // both predicates assign the variable but differ on the value
                        return Nonexistant;
                    } else {
                        // both predicates assign the variable to the same value
                        s = s_it.next();
                        o = o_it.next();
                    }
                }
            }
        }
        // Observed zero inconsistencies. A unified predicate exists...
        match [s_not_o.is_empty(), o_not_s.is_empty()] {
            [true, true] => Equivalent,       // ... equivalent to both.
            [false, true] => FormerNotLatter, // ... equivalent to self.
            [true, false] => LatterNotFormer, // ... equivalent to other.
            [false, false] => {
                // ... which is the union of the predicates' assignments but
                //     is equivalent to neither self nor other.
                let mut new = self.clone();
                for (&id, &b) in o_not_s {
                    new.assigned.insert(id, b);
                }
                New(new)
            }
        }
    }

    pub fn iter_matching(&self, value: bool) -> impl Iterator<Item = ChannelId> + '_ {
        self.assigned
            .iter()
            .filter_map(move |(&channel_id, &b)| if b == value { Some(channel_id) } else { None })
    }

    pub fn batch_assign_nones(
        &mut self,
        channel_ids: impl Iterator<Item = ChannelId>,
        value: bool,
    ) {
        for channel_id in channel_ids {
            self.assigned.entry(channel_id).or_insert(value);
        }
    }
    pub fn replace_assignment(&mut self, channel_id: ChannelId, value: bool) -> Option<bool> {
        self.assigned.insert(channel_id, value)
    }
    pub fn union_with(&self, other: &Self) -> Option<Self> {
        let mut res = self.clone();
        for (&channel_id, &assignment_1) in other.assigned.iter() {
            match res.assigned.insert(channel_id, assignment_1) {
                Some(assignment_2) if assignment_1 != assignment_2 => return None,
                _ => {}
            }
        }
        Some(res)
    }
    pub fn query(&self, x: ChannelId) -> Option<bool> {
        self.assigned.get(&x).copied()
    }
    pub fn new_trivial() -> Self {
        Self { assigned: Default::default() }
    }
}
impl Debug for Predicate {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.pad("{")?;
        for (ChannelId { controller_id, channel_index }, &v) in self.assigned.iter() {
            f.write_fmt(format_args!(
                "({:?},{:?})=>{}, ",
                controller_id,
                channel_index,
                if v { 'T' } else { 'F' }
            ))?
        }
        f.pad("}")
    }
}

#[test]
fn pred_sat() {
    use maplit::btreemap;
    let mut c = ChannelIdStream::new(0);
    let ch = std::iter::repeat_with(move || c.next()).take(5).collect::<Vec<_>>();
    let p = Predicate::new_trivial();
    let p_0t = Predicate { assigned: btreemap! { ch[0] => true } };
    let p_0f = Predicate { assigned: btreemap! { ch[0] => false } };
    let p_0f_3f = Predicate { assigned: btreemap! { ch[0] => false, ch[3] => false } };
    let p_0f_3t = Predicate { assigned: btreemap! { ch[0] => false, ch[3] => true } };

    assert!(p.satisfies(&p));
    assert!(p_0t.satisfies(&p_0t));
    assert!(p_0f.satisfies(&p_0f));
    assert!(p_0f_3f.satisfies(&p_0f_3f));
    assert!(p_0f_3t.satisfies(&p_0f_3t));

    assert!(p_0t.satisfies(&p));
    assert!(p_0f.satisfies(&p));
    assert!(p_0f_3f.satisfies(&p_0f));
    assert!(p_0f_3t.satisfies(&p_0f));

    assert!(!p.satisfies(&p_0t));
    assert!(!p.satisfies(&p_0f));
    assert!(!p_0f.satisfies(&p_0t));
    assert!(!p_0t.satisfies(&p_0f));
    assert!(!p_0f_3f.satisfies(&p_0f_3t));
    assert!(!p_0f_3t.satisfies(&p_0f_3f));
    assert!(!p_0t.satisfies(&p_0f_3f));
    assert!(!p_0f.satisfies(&p_0f_3f));
    assert!(!p_0t.satisfies(&p_0f_3t));
    assert!(!p_0f.satisfies(&p_0f_3t));
}

#[test]
fn pred_common_sat() {
    use maplit::btreemap;
    use CommonSatResult::*;

    let mut c = ChannelIdStream::new(0);
    let ch = std::iter::repeat_with(move || c.next()).take(5).collect::<Vec<_>>();
    let p = Predicate::new_trivial();
    let p_0t = Predicate { assigned: btreemap! { ch[0] => true } };
    let p_0f = Predicate { assigned: btreemap! { ch[0] => false } };
    let p_3f = Predicate { assigned: btreemap! { ch[3] => false } };
    let p_0f_3f = Predicate { assigned: btreemap! { ch[0] => false, ch[3] => false } };
    let p_0f_3t = Predicate { assigned: btreemap! { ch[0] => false, ch[3] => true } };

    assert_eq![p.common_satisfier(&p), Equivalent];
    assert_eq![p_0t.common_satisfier(&p_0t), Equivalent];

    assert_eq![p.common_satisfier(&p_0t), LatterNotFormer];
    assert_eq![p_0t.common_satisfier(&p), FormerNotLatter];

    assert_eq![p_0t.common_satisfier(&p_0f), Nonexistant];
    assert_eq![p_0f_3t.common_satisfier(&p_0f_3f), Nonexistant];
    assert_eq![p_0f_3t.common_satisfier(&p_3f), Nonexistant];
    assert_eq![p_3f.common_satisfier(&p_0f_3t), Nonexistant];

    assert_eq![p_0f.common_satisfier(&p_3f), New(p_0f_3f)];
}
