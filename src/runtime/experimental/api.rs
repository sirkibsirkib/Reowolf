use crate::common::*;
use crate::runtime::endpoint::Endpoint;
use crate::runtime::endpoint::EndpointExt;
use crate::runtime::endpoint::EndpointInfo;

use std::net::SocketAddr;
use std::sync::Arc;

pub enum Polarity {
    In,
    Out,
}
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
#[repr(C)]
pub struct Port(pub u32);
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
pub struct InPort(Port);
pub struct OutPort(Port);

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
trait Binds<T> {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> T;
}
impl Binds<InPort> for Connecting {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> InPort {
        self.bindings.push((coupling, Polarity::In, addr).into());
        let pid: u32 = (self.bindings.len() - 1).try_into().expect("Port ID overflow!");
        InPort(Port(pid))
    }
}
impl Binds<OutPort> for Connecting {
    fn bind(&mut self, coupling: Coupling, addr: SocketAddr) -> OutPort {
        self.bindings.push((coupling, Polarity::Out, addr).into());
        let pid: u32 = (self.bindings.len() - 1).try_into().expect("Port ID overflow!");
        OutPort(Port(pid))
    }
}
impl Connecting {
    pub fn connect(&mut self, _timeout: Option<Duration>) -> Result<Connected, ()> {
        let controller_id = 42;
        let channel_index_stream = ChannelIndexStream::default();
        let native_ports = (0..self.bindings.len()).map(|x| Port(x as u32)).collect();
        self.bindings.clear();
        Ok(Connected {
            controller_id,
            channel_index_stream,
            components: vec![],
            endpoint_exts: vec![],
            native_ports,
        })
    }
}
pub struct Protocol;
impl Protocol {
    pub fn parse(_pdl_text: &[u8]) -> Result<Self, ()> {
        Ok(Protocol)
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
    pub fn new_channel(&mut self) -> (OutPort, InPort) {
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
        for p in ports.iter() {
            self.native_ports.insert(Port(p.0));
        }
        (OutPort(ports[0]), InPort(ports[1]))
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
            use super::bits::BitChunkIter;
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
    let mut c = c.connect(None).unwrap();
    let (mem_out, mem_in) = c.new_channel();
    let mut inbuf = [0u8; 64];
    c.new_component(&proto_0, b"sync".to_vec(), &[net_in.into(), mem_out.into()]).unwrap();
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
    msgbuf: *mut u8,
    buflen: usize,
    msglen: usize,
    optional: bool,
}

pub enum PortOpRs<'a> {
    In { msg_range: Option<Range<usize>>, port: &'a InPort },
    Out { msg: &'a [u8], port: &'a OutPort, optional: bool },
}
pub struct InPortOp<'a> {
    msg_range: Option<Range<usize>>, // written by sync
    port: &'a InPort,
}
pub struct OutPortOp<'a> {
    msg: &'a [u8],
    port: &'a OutPort,
    optional: bool,
}
