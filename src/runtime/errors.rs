use crate::common::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PortBindErr {
    AlreadyConnected,
    IndexOutOfBounds,
    NotConfigured,
    ParseErr,
    AlreadyConfigured,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReadGottenErr {
    NotConnected,
    IndexOutOfBounds,
    WrongPolarity,
    NoPreviousRound,
    DidNotGet,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PortOpErr {
    IndexOutOfBounds,
    NotConnected,
    WrongPolarity,
    DuplicateOperation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigErr {
    AlreadyConnected,
    ParseErr(String),
    AlreadyConfigured,
    NoSuchComponent,
    NonPortTypeParameters,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConnectErr {
    PortNotBound { native_index: usize },
    NotConfigured,
    AlreadyConnected,
    MetaProtocolDeviation,
    Disconnected,
    PollInitFailed,
    MessengerRecvErr(MessengerRecvErr),
    Timeout,
    PollingFailed,
    PolarityMatched(SocketAddr),
    AcceptFailed(SocketAddr),
    PassiveConnectFailed(SocketAddr),
    BindFailed(SocketAddr),
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PollDeadlineErr {
    PollingFailed,
    Timeout,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EndpointErr {
    Disconnected,
    MetaProtocolDeviation,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SyncErr {
    NotConnected,
    MessengerRecvErr(MessengerRecvErr),
    Inconsistent,
    Timeout,
    ElaborateFromNonChild,
    AnnounceFromNonParent,
    PayloadPremiseExcludesTheChannel(ChannelId),
    UnexpectedSetupMsg,
    EndpointErr(EndpointErr),
    EvalErr(EvalErr),
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EvalErr {
    ComponentExitWhileBranching,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessengerRecvErr {
    PollingFailed,
    EndpointErr(EndpointErr),
}
impl From<MainComponentErr> for ConfigErr {
    fn from(e: MainComponentErr) -> Self {
        use ConfigErr as C;
        use MainComponentErr as M;
        match e {
            M::NoSuchComponent => C::NoSuchComponent,
            M::NonPortTypeParameters => C::NonPortTypeParameters,
        }
    }
}
