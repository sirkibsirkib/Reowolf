use crate::common::*;
use crate::runtime::{errors::*, Predicate};
use mio::{Evented, PollOpt, Ready};

pub(crate) enum Endpoint {
    Memory { s: mio_extras::channel::Sender<Msg>, r: mio_extras::channel::Receiver<Msg> },
    Network(NetworkEndpoint),
}

#[derive(Debug)]
pub(crate) struct EndpointExt {
    pub endpoint: Endpoint,
    pub info: EndpointInfo,
}
#[derive(Debug, Copy, Clone)]
pub struct EndpointInfo {
    pub polarity: Polarity,
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone)]
pub(crate) enum Decision {
    Failure,
    Success(Predicate),
}

#[derive(Clone, Debug)]
pub(crate) enum Msg {
    SetupMsg(SetupMsg),
    CommMsg(CommMsg),
}
#[derive(Clone, Debug)]
pub(crate) enum SetupMsg {
    // sent by the passive endpoint to the active endpoint
    ChannelSetup { info: EndpointInfo },
    LeaderEcho { maybe_leader: ControllerId },
    LeaderAnnounce { leader: ControllerId },
    YouAreMyParent,
}
impl Into<Msg> for SetupMsg {
    fn into(self) -> Msg {
        Msg::SetupMsg(self)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommMsg {
    pub round_index: usize,
    pub contents: CommMsgContents,
}
#[derive(Clone, Debug)]
pub(crate) enum CommMsgContents {
    SendPayload { payload_predicate: Predicate, payload: Payload },
    Elaborate { partial_oracle: Predicate }, // SINKWARD
    Failure,                                 // SINKWARD
    Announce { decision: Decision },         // SINKAWAYS
}

pub struct NetworkEndpoint {
    stream: mio::net::TcpStream,
    inbox: Vec<u8>,
    outbox: Vec<u8>,
}

impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let s = match self {
            Endpoint::Memory { .. } => "Memory",
            Endpoint::Network(..) => "Network",
        };
        f.write_fmt(format_args!("Endpoint::{}", s))
    }
}

impl CommMsgContents {
    pub fn into_msg(self, round_index: usize) -> Msg {
        Msg::CommMsg(CommMsg { round_index, contents: self })
    }
}

impl From<EndpointErr> for ConnectErr {
    fn from(e: EndpointErr) -> Self {
        match e {
            EndpointErr::Disconnected => ConnectErr::Disconnected,
            EndpointErr::MetaProtocolDeviation => ConnectErr::MetaProtocolDeviation,
        }
    }
}
impl Endpoint {
    // asymmetric
    // pub(crate) fn from_fresh_stream(stream: mio::net::TcpStream) -> Self {
    //     Self::Network(NetworkEndpoint { stream, inbox: vec![], outbox: vec![] })
    // }
    pub(crate) fn from_fresh_stream_and_inbox(stream: mio::net::TcpStream, inbox: Vec<u8>) -> Self {
        Self::Network(NetworkEndpoint { stream, inbox, outbox: vec![] })
    }

    // symmetric
    pub fn new_memory_pair() -> [Self; 2] {
        let (s1, r1) = mio_extras::channel::channel::<Msg>();
        let (s2, r2) = mio_extras::channel::channel::<Msg>();
        [Self::Memory { s: s1, r: r2 }, Self::Memory { s: s2, r: r1 }]
    }
    pub fn send(&mut self, msg: Msg) -> Result<(), EndpointErr> {
        match self {
            Self::Memory { s, .. } => s.send(msg).map_err(|_| EndpointErr::Disconnected),
            Self::Network(NetworkEndpoint { stream, outbox, .. }) => {
                use crate::runtime::serde::Ser;
                outbox.ser(&msg).expect("ser failed");
                loop {
                    use std::io::Write;
                    match stream.write(outbox) {
                        Ok(0) => return Ok(()),
                        Ok(bytes_written) => {
                            outbox.drain(0..bytes_written);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            panic!("sending shouldn't WouldBlock")
                        }
                        Err(_e) => return Err(EndpointErr::Disconnected),
                    }
                }
            }
        }
    }
    pub fn recv(&mut self) -> Result<Option<Msg>, EndpointErr> {
        match self {
            Self::Memory { r, .. } => match r.try_recv() {
                Ok(msg) => Ok(Some(msg)),
                Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(EndpointErr::Disconnected),
            },
            Self::Network(NetworkEndpoint { stream, inbox, .. }) => {
                // populate inbox as much as possible
                'read_loop: loop {
                    use std::io::Read;
                    match stream.read_to_end(inbox) {
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break 'read_loop,
                        Ok(0) => break 'read_loop,
                        Ok(_) => (),
                        Err(_e) => return Err(EndpointErr::Disconnected),
                    }
                }
                use crate::runtime::serde::{De, MonitoredReader};
                let mut monitored = MonitoredReader::from(&inbox[..]);
                match De::<Msg>::de(&mut monitored) {
                    Ok(msg) => {
                        let msg_size2 = monitored.bytes_read();
                        inbox.drain(0..(msg_size2.try_into().unwrap()));
                        Ok(Some(msg))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
                    Err(_) => Err(EndpointErr::MetaProtocolDeviation),
                }
            }
        }
    }
}

impl Evented for Endpoint {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), std::io::Error> {
        match self {
            Self::Memory { r, .. } => r.register(poll, token, interest, opts),
            Self::Network(n) => n.register(poll, token, interest, opts),
        }
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), std::io::Error> {
        match self {
            Self::Memory { r, .. } => r.reregister(poll, token, interest, opts),
            Self::Network(n) => n.reregister(poll, token, interest, opts),
        }
    }

    fn deregister(&self, poll: &Poll) -> Result<(), std::io::Error> {
        match self {
            Self::Memory { r, .. } => r.deregister(poll),
            Self::Network(n) => n.deregister(poll),
        }
    }
}

impl Evented for NetworkEndpoint {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), std::io::Error> {
        self.stream.register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), std::io::Error> {
        self.stream.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> Result<(), std::io::Error> {
        self.stream.deregister(poll)
    }
}
