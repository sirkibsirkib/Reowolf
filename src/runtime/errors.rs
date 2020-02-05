use crate::common::*;

#[derive(Debug)]
pub enum PortBindErr {
    AlreadyConnected,
    IndexOutOfBounds,
    NotConfigured,
    ParseErr,
    AlreadyConfigured,
}
#[derive(Debug)]
pub enum ReadGottenErr {
    NotConnected,
    IndexOutOfBounds,
    WrongPolarity,
    NoPreviousRound,
    DidntGet,
}
#[derive(Debug)]
pub enum PortOpErr {
    IndexOutOfBounds,
    NotConnected,
    WrongPolarity,
    DuplicateOperation,
}
#[derive(Debug)]
pub enum ConfigErr {
    AlreadyConnected,
    ParseErr(String),
    AlreadyConfigured,
}
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum PollDeadlineErr {
    PollingFailed,
    Timeout,
}

#[derive(Debug, Clone)]
pub enum EndpointErr {
    Disconnected,
    MetaProtocolDeviation,
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum EvalErr {
    ComponentExitWhileBranching,
}
#[derive(Debug, Clone)]
pub enum MessengerRecvErr {
    PollingFailed,
    EndpointErr(EndpointErr),
}
