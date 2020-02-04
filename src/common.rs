///////////////////// PRELUDE /////////////////////

pub use core::{
    cmp::Ordering,
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Range, RangeFrom},
    time::Duration,
};
pub use indexmap::{IndexMap, IndexSet};
pub use maplit::{hashmap, hashset};
pub use mio::{
    net::{TcpListener, TcpStream},
    Event, Evented, Events, Poll, PollOpt, Ready, Token,
};
pub use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    convert::TryInto,
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};
pub use Polarity::*;

///////////////////// DEFS /////////////////////

pub type Payload = Vec<u8>;
pub type ControllerId = u32;
pub type ChannelIndex = u32;

/// This is a unique identifier for a channel (i.e., port).
#[derive(Debug, Eq, PartialEq, Clone, Hash, Copy, Ord, PartialOrd)]
pub struct ChannelId {
    pub(crate) controller_id: ControllerId,
    pub(crate) channel_index: ChannelIndex,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash, Copy, Ord, PartialOrd)]
pub enum Polarity {
    Putter, // output port (from the perspective of the component)
    Getter, // input port (from the perspective of the component)
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Debug)]
pub struct Key(u64);

pub trait ProtocolDescription: Sized {
    type S: ComponentState<D = Self>;

    fn parse(pdl: &[u8]) -> Result<Self, String>;
    fn main_interface_polarities(&self) -> Vec<Polarity>;
    fn new_main_component(&self, interface: &[Key]) -> Self::S;
}

pub trait ComponentState: Sized + Clone {
    type D: ProtocolDescription;
    fn pre_sync_run<C: MonoContext<D = Self::D, S = Self>>(
        &mut self,
        runtime_ctx: &mut C,
        protocol_description: &Self::D,
    ) -> MonoBlocker;

    fn sync_run<C: PolyContext<D = Self::D>>(
        &mut self,
        runtime_ctx: &mut C,
        protocol_description: &Self::D,
    ) -> PolyBlocker;
}

#[derive(Debug, Clone)]
pub enum MonoBlocker {
    Inconsistent,
    ComponentExit,
    SyncBlockStart,
}

#[derive(Debug, Clone)]
pub enum PolyBlocker {
    Inconsistent,
    SyncBlockEnd,
    CouldntReadMsg(Key),
    CouldntCheckFiring(Key),
    PutMsg(Key, Payload),
}

pub trait MonoContext {
    type D: ProtocolDescription;
    type S: ComponentState<D = Self::D>;

    fn new_component(&mut self, moved_keys: HashSet<Key>, init_state: Self::S);
    fn new_channel(&mut self) -> [Key; 2];
    fn new_random(&self) -> u64;
}
pub trait PolyContext {
    type D: ProtocolDescription;

    fn is_firing(&self, ekey: Key) -> Option<bool>;
    fn read_msg(&self, ekey: Key) -> Option<&Payload>;
}

///////////////////// IMPL /////////////////////
impl Key {
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
    pub fn to_raw(self) -> u64 {
        self.0
    }
    pub fn to_token(self) -> mio::Token {
        mio::Token(self.0.try_into().unwrap())
    }
    pub fn from_token(t: mio::Token) -> Self {
        Self(t.0.try_into().unwrap())
    }
}
