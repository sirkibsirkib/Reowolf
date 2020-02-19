use crate::common::*;
use crate::runtime::endpoint::Endpoint;
use crate::runtime::endpoint::EndpointExt;
use crate::runtime::endpoint::EndpointInfo;

use std::net::SocketAddr;
use std::sync::Arc;

pub enum Coupling {
    Active,
    Passive,
}
pub struct Binding {
    pub coupling: Coupling,
    pub polarity: Polarity,
    pub addr: SocketAddr,
}
impl From<(Coupling, Polarity, SocketAddr)> for Binding {
    fn from((coupling, polarity, addr): (Coupling, Polarity, SocketAddr)) -> Self {
        Self { coupling, polarity, addr }
    }
}

pub struct MsgBuffer<'a> {
    slice: &'a mut [u8],
    len: usize,
}
impl MsgBuffer<'_> {
    pub fn clear(&mut self) {
        self.len = 0;
    }
    pub fn write_msg(&mut self, r: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        self.slice.write_all(r)?;
        self.len = r.len();
        Ok(())
    }
    pub fn read_msg(&self) -> &[u8] {
        &self.slice[0..self.len]
    }
}
impl<'a> From<&'a mut [u8]> for MsgBuffer<'a> {
    fn from(slice: &'a mut [u8]) -> Self {
        Self { slice, len: 0 }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Port(pub u32);
pub struct PortOp<'a> {
    pub port: Port,
    pub msg: Option<&'a [u8]>,
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
    bindings: Vec<Binding>, // invariant: no more than std::u32::MAX entries
}
impl Connecting {
    pub fn bind(&mut self, binding: Binding) -> Port {
        self.bindings.push(binding);
        // preserve invariant
        let pid: u32 = (self.bindings.len() - 1).try_into().expect("Port ID overflow!");
        Port(pid)
    }
    pub fn connect(&mut self, timeout: Option<Duration>) -> Result<Connected, ()> {
        let controller_id = 42;
        let channel_index_stream = ChannelIndexStream::default();
        // drain self if successful
        todo!()
    }
}
pub struct Protocol;
impl Protocol {
    pub fn parse(_pdl_text: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}
struct ComponentExt {
    protocol: Arc<Protocol>,
    ports: HashSet<Port>,
    name: Vec<u8>,
}
pub struct Connected {
    native_ports: HashSet<Port>,
    controller_id: ControllerId,
    channel_index_stream: ChannelIndexStream,
    endpoint_exts: Vec<EndpointExt>, // invaraint
    components: Vec<ComponentExt>,
}
impl Connected {
    pub fn new_channel(&mut self) -> [Port; 2] {
        assert!(self.endpoint_exts.len() <= std::u32::MAX as usize - 2);
        let ports =
            [Port(self.endpoint_exts.len() as u32 - 1), Port(self.endpoint_exts.len() as u32)];
        let channel_id = ChannelId {
            controller_id: self.controller_id,
            channel_index: self.channel_index_stream.next(),
        };
        let [e0, e1] = Endpoint::new_memory_pair();
        self.endpoint_exts.push(EndpointExt {
            info: EndpointInfo { channel_id, polarity: Putter },
            endpoint: e0,
        });
        self.endpoint_exts.push(EndpointExt {
            info: EndpointInfo { channel_id, polarity: Getter },
            endpoint: e1,
        });
        ports
    }
    pub fn new_component(
        &mut self,
        protocol: &Arc<Protocol>,
        name: Vec<u8>,
        moved_ports: &[Port],
    ) -> Result<(), ()> {
        let moved_ports = moved_ports.iter().copied().collect();
        if !self.native_ports.is_superset(&moved_ports) {
            return Err(());
        }
        self.native_ports.retain(|e| !moved_ports.contains(e));
        self.components.push(ComponentExt { ports: moved_ports, protocol: protocol.clone(), name });
        // TODO add a singleton machine
        Ok(())
    }
    pub fn sync_set(&mut self, ops: &mut [PortOp]) {
        todo!()
    }
    pub fn sync_subsets(
        &mut self,
        _ops: &mut [PortOp],
        bit_subsets: &[&[usize]],
    ) -> Result<usize, ()> {
        for &bit_subset in bit_subsets {
            use super::bits::BitChunkIter;
            BitChunkIter::new(bit_subset.iter().copied());
        }
        todo!()
    }
}

#[test]
fn test() {
    let mut c = Connecting::default();
    let p0 = c.bind(Binding {
        coupling: Coupling::Active,
        polarity: Putter,
        addr: "127.0.0.1:8000".parse().unwrap(),
    });
    let p1 = c.bind(Binding {
        coupling: Coupling::Passive,
        polarity: Putter,
        addr: "127.0.0.1:8001".parse().unwrap(),
    });

    let proto_0 = Arc::new(Protocol::parse(b"").unwrap());
    let mut c = c.connect(None).unwrap();
    let [p2, p3] = c.new_channel();
    c.new_component(&proto_0, b"sync".to_vec(), &[p0, p2]).unwrap();
    let mut ops = [
        //
        PortOp { port: p1, msg: Some(b"hi!") },
        PortOp { port: p1, msg: Some(b"ahoy!") },
        PortOp { port: p1, msg: Some(b"hello!") },
    ];
    c.sync_subsets(&mut ops, &[&[0b001], &[0b010], &[0b100]]).unwrap();
}
