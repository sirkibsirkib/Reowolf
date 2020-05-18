use crate::common::*;
use crate::runtime::{
    endpoint::{CommMsg, CommMsgContents, Decision, EndpointInfo, Msg, SetupMsg},
    Predicate,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{ErrorKind::InvalidData, Read, Write};

pub trait Ser<T>: Write {
    fn ser(&mut self, t: &T) -> Result<(), std::io::Error>;
}
pub trait De<T>: Read {
    fn de(&mut self) -> Result<T, std::io::Error>;
}

pub struct MonitoredReader<R: Read> {
    bytes: usize,
    r: R,
}
impl<R: Read> From<R> for MonitoredReader<R> {
    fn from(r: R) -> Self {
        Self { r, bytes: 0 }
    }
}
impl<R: Read> MonitoredReader<R> {
    pub fn bytes_read(&self) -> usize {
        self.bytes
    }
}
impl<R: Read> Read for MonitoredReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let n = self.r.read(buf)?;
        self.bytes += n;
        Ok(n)
    }
}

/////////////////////////////////////////

struct VarLenInt(u64);

macro_rules! ser_seq {
    ( $w:expr ) => {{
        io::Result::Ok(())
    }};
    ( $w:expr, $first:expr ) => {{
        $w.ser($first)
    }};
    ( $w:expr, $first:expr, $( $x:expr ),+ ) => {{
        $w.ser($first)?;
        ser_seq![$w, $( $x ),*]
    }};
}
/////////////////////////////////////////

impl<W: Write> Ser<bool> for W {
    fn ser(&mut self, t: &bool) -> Result<(), std::io::Error> {
        self.ser(&match t {
            true => b'T',
            false => b'F',
        })
    }
}
impl<R: Read> De<bool> for R {
    fn de(&mut self) -> Result<bool, std::io::Error> {
        let b: u8 = self.de()?;
        Ok(match b {
            b'T' => true,
            b'F' => false,
            _ => return Err(InvalidData.into()),
        })
    }
}

impl<W: Write> Ser<u8> for W {
    fn ser(&mut self, t: &u8) -> Result<(), std::io::Error> {
        self.write_u8(*t)
    }
}
impl<R: Read> De<u8> for R {
    fn de(&mut self) -> Result<u8, std::io::Error> {
        self.read_u8()
    }
}

impl<W: Write> Ser<u16> for W {
    fn ser(&mut self, t: &u16) -> Result<(), std::io::Error> {
        self.write_u16::<BigEndian>(*t)
    }
}
impl<R: Read> De<u16> for R {
    fn de(&mut self) -> Result<u16, std::io::Error> {
        self.read_u16::<BigEndian>()
    }
}

impl<W: Write> Ser<u32> for W {
    fn ser(&mut self, t: &u32) -> Result<(), std::io::Error> {
        self.write_u32::<BigEndian>(*t)
    }
}
impl<R: Read> De<u32> for R {
    fn de(&mut self) -> Result<u32, std::io::Error> {
        self.read_u32::<BigEndian>()
    }
}

impl<W: Write> Ser<u64> for W {
    fn ser(&mut self, t: &u64) -> Result<(), std::io::Error> {
        self.write_u64::<BigEndian>(*t)
    }
}
impl<R: Read> De<u64> for R {
    fn de(&mut self) -> Result<u64, std::io::Error> {
        self.read_u64::<BigEndian>()
    }
}

impl<W: Write> Ser<Payload> for W {
    fn ser(&mut self, t: &Payload) -> Result<(), std::io::Error> {
        self.ser(&VarLenInt(t.len() as u64))?;
        for byte in t.as_slice() {
            self.ser(byte)?;
        }
        Ok(())
    }
}
impl<R: Read> De<Payload> for R {
    fn de(&mut self) -> Result<Payload, std::io::Error> {
        let VarLenInt(len) = self.de()?;
        let mut x = Vec::with_capacity(len as usize);
        for _ in 0..len {
            x.push(self.de()?);
        }
        Ok(x.into())
    }
}

impl<W: Write> Ser<VarLenInt> for W {
    fn ser(&mut self, t: &VarLenInt) -> Result<(), std::io::Error> {
        integer_encoding::VarIntWriter::write_varint(self, t.0).map(|_| ())
    }
}
impl<R: Read> De<VarLenInt> for R {
    fn de(&mut self) -> Result<VarLenInt, std::io::Error> {
        integer_encoding::VarIntReader::read_varint(self).map(VarLenInt)
    }
}

impl<W: Write> Ser<ChannelId> for W {
    fn ser(&mut self, t: &ChannelId) -> Result<(), std::io::Error> {
        self.ser(&t.controller_id)?;
        self.ser(&VarLenInt(t.channel_index as u64))
    }
}
impl<R: Read> De<ChannelId> for R {
    fn de(&mut self) -> Result<ChannelId, std::io::Error> {
        Ok(ChannelId {
            controller_id: self.de()?,
            channel_index: De::<VarLenInt>::de(self)?.0 as ChannelIndex,
        })
    }
}

impl<W: Write> Ser<Predicate> for W {
    fn ser(&mut self, t: &Predicate) -> Result<(), std::io::Error> {
        self.ser(&VarLenInt(t.assigned.len() as u64))?;
        for (channel_id, boolean) in &t.assigned {
            ser_seq![self, channel_id, boolean]?;
        }
        Ok(())
    }
}
impl<R: Read> De<Predicate> for R {
    fn de(&mut self) -> Result<Predicate, std::io::Error> {
        let VarLenInt(len) = self.de()?;
        let mut assigned = BTreeMap::<ChannelId, bool>::default();
        for _ in 0..len {
            assigned.insert(self.de()?, self.de()?);
        }
        Ok(Predicate { assigned })
    }
}
impl<W: Write> Ser<Decision> for W {
    fn ser(&mut self, t: &Decision) -> Result<(), std::io::Error> {
        match t {
            Decision::Failure => self.ser(&b'F'),
            Decision::Success(predicate) => {
                self.ser(&b'S')?;
                self.ser(predicate)
            }
        }
    }
}
impl<R: Read> De<Decision> for R {
    fn de(&mut self) -> Result<Decision, std::io::Error> {
        let b: u8 = self.de()?;
        Ok(match b {
            b'F' => Decision::Failure,
            b'S' => Decision::Success(self.de()?),
            _ => return Err(InvalidData.into()),
        })
    }
}

impl<W: Write> Ser<Polarity> for W {
    fn ser(&mut self, t: &Polarity) -> Result<(), std::io::Error> {
        self.ser(&match t {
            Polarity::Putter => b'P',
            Polarity::Getter => b'G',
        })
    }
}
impl<R: Read> De<Polarity> for R {
    fn de(&mut self) -> Result<Polarity, std::io::Error> {
        let b: u8 = self.de()?;
        Ok(match b {
            b'P' => Polarity::Putter,
            b'G' => Polarity::Getter,
            _ => return Err(InvalidData.into()),
        })
    }
}

impl<W: Write> Ser<EndpointInfo> for W {
    fn ser(&mut self, t: &EndpointInfo) -> Result<(), std::io::Error> {
        let EndpointInfo { channel_id, polarity } = t;
        ser_seq![self, channel_id, polarity]
    }
}
impl<R: Read> De<EndpointInfo> for R {
    fn de(&mut self) -> Result<EndpointInfo, std::io::Error> {
        Ok(EndpointInfo { channel_id: self.de()?, polarity: self.de()? })
    }
}

impl<W: Write> Ser<Msg> for W {
    fn ser(&mut self, t: &Msg) -> Result<(), std::io::Error> {
        use {CommMsgContents::*, SetupMsg::*};
        match t {
            Msg::SetupMsg(s) => match s {
                // [flag, data]
                ChannelSetup { info } => ser_seq![self, &0u8, info],
                LeaderEcho { maybe_leader } => ser_seq![self, &1u8, maybe_leader],
                LeaderAnnounce { leader } => ser_seq![self, &2u8, leader],
                YouAreMyParent => ser_seq![self, &3u8],
            },
            Msg::CommMsg(CommMsg { round_index, contents }) => {
                // [flag, round_num, data]
                let varlenint = &VarLenInt(*round_index as u64);
                match contents {
                    SendPayload { payload_predicate, payload } => {
                        ser_seq![self, &4u8, varlenint, payload_predicate, payload]
                    }
                    Elaborate { partial_oracle } => ser_seq![self, &5u8, varlenint, partial_oracle],
                    Announce { decision } => ser_seq![self, &6u8, varlenint, decision],
                    Failure => ser_seq![self, &7u8, varlenint],
                }
            }
        }
    }
}
impl<R: Read> De<Msg> for R {
    fn de(&mut self) -> Result<Msg, std::io::Error> {
        use {CommMsgContents::*, SetupMsg::*};
        let b: u8 = self.de()?;
        Ok(match b {
            0..=3 => Msg::SetupMsg(match b {
                // [flag, data]
                0u8 => ChannelSetup { info: self.de()? },
                1u8 => LeaderEcho { maybe_leader: self.de()? },
                2u8 => LeaderAnnounce { leader: self.de()? },
                3u8 => YouAreMyParent,
                _ => unreachable!(),
            }),
            4..=7 => {
                // [flag, round_num, data]
                let VarLenInt(varlenint) = self.de()?;
                let contents = match b {
                    4u8 => SendPayload { payload_predicate: self.de()?, payload: self.de()? },
                    5u8 => Elaborate { partial_oracle: self.de()? },
                    6u8 => Announce { decision: self.de()? },
                    7u8 => Failure,
                    _ => unreachable!(),
                };
                Msg::CommMsg(CommMsg { round_index: varlenint as usize, contents })
            }
            _ => return Err(InvalidData.into()),
        })
    }
}
