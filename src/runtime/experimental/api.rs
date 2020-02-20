use crate::common::*;
use crate::runtime::endpoint::Endpoint;
use crate::runtime::endpoint::EndpointExt;
use crate::runtime::endpoint::EndpointInfo;
use core::mem::MaybeUninit;
use std::collections::BTreeSet;

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

// data contains values in one of three states:
// 1. occupied: ininitialized. will be dropped.
// 2. vacant: uninitialized. may be reused implicitly. won't be dropped.
// 2. reserved: uninitialized. may be occupied implicitly. won't be dropped.
struct VecStorage<T> {
    // invariant A: elements at indices (0..data.len()) / vacant / reserved are occupied
    // invariant B: reserved & vacant = {}
    // invariant C: (vacant U reserved) subset of (0..data.len)
    data: Vec<MaybeUninit<T>>,
    vacant: BTreeSet<usize>,
    reserved: BTreeSet<usize>,
}
impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        Self { data: Default::default(), vacant: Default::default(), reserved: Default::default() }
    }
}
impl<T> Debug for VecStorage<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        enum FmtT<'a, T> {
            Vacant,
            Reserved,
            Occupied(&'a T),
        };
        impl<T> Debug for FmtT<'_, T>
        where
            T: Debug,
        {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                match self {
                    FmtT::Vacant => write!(f, "Vacant"),
                    FmtT::Reserved => write!(f, "Reserved"),
                    FmtT::Occupied(t) => write!(f, "Occupied({:?})", t),
                }
            }
        }
        let iter = (0..self.data.len()).map(|i| {
            if self.vacant.contains(&i) {
                FmtT::Vacant
            } else if self.reserved.contains(&i) {
                FmtT::Reserved
            } else {
                // 2. Invariant A => reading valid ata
                unsafe {
                    // 1. index is within bounds
                    // 2. i is occupied => initialized data is being dropped
                    FmtT::Occupied(&*self.data.get_unchecked(i).as_ptr())
                }
            }
        });
        f.debug_list().entries(iter).finish()
    }
}
impl<T> Drop for VecStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}
impl<T> VecStorage<T> {
    // ASSUMES that i in 0..self.data.len()
    unsafe fn get_occupied_unchecked(&self, i: usize) -> Option<&T> {
        if self.vacant.contains(&i) || self.reserved.contains(&i) {
            None
        } else {
            // 2. Invariant A => reading valid ata
            Some(&*self.data.get_unchecked(i).as_ptr())
        }
    }
    // ASSUMES that i in 0..self.data.len()
    unsafe fn get_mut_occupied_unchecked(&mut self, i: usize) -> Option<&mut T> {
        if self.vacant.contains(&i) || self.reserved.contains(&i) {
            None
        } else {
            // 2. Invariant A => reading valid ata
            Some(&mut *self.data.get_unchecked_mut(i).as_mut_ptr())
        }
    }
    // breaks invariant A: returned index is in NO state
    fn pop_vacant(&mut self) -> usize {
        if let Some(i) = pop_set_arb(&mut self.vacant) {
            i
        } else {
            self.data.push(MaybeUninit::uninit());
            self.data.len() - 1
        }
    }
    //////////////
    pub fn clear(&mut self) {
        for i in 0..self.data.len() {
            if !self.vacant.contains(&i) && !self.reserved.contains(&i) {
                // invariant A: this element is OCCUPIED
                unsafe {
                    // 1. by construction, i is in bounds
                    // 2. i is occupied => initialized data is being dropped
                    drop(self.data.get_unchecked_mut(i).as_ptr().read());
                }
            }
        }
        self.vacant.clear();
        self.reserved.clear();
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.data.len()).filter_map(move |i| unsafe { self.get_occupied_unchecked(i) })
    }
    pub fn get_occupied(&self, i: usize) -> Option<&T> {
        if i >= self.data.len() {
            None
        } else {
            unsafe {
                // index is within bounds
                self.get_occupied_unchecked(i)
            }
        }
    }
    pub fn get_mut_occupied(&mut self, i: usize) -> Option<&mut T> {
        if i >= self.data.len() {
            None
        } else {
            unsafe {
                // index is within bounds
                self.get_mut_occupied_unchecked(i)
            }
        }
    }
    pub fn new_reserved(&mut self) -> usize {
        let i = self.pop_vacant(); // breaks invariant A: i is in NO state
        self.reserved.insert(i); // restores invariant A
        i
    }
    pub fn occupy_reserved(&mut self, i: usize, t: T) {
        assert!(self.reserved.remove(&i)); // breaks invariant A
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t)
            // restores invariant A
        };
    }
    pub fn new_occupied(&mut self, t: T) -> usize {
        let i = self.pop_vacant(); // breaks invariant A: i is in NO state
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t)
            // restores invariant A
        };
        i
    }
    pub fn vacate(&mut self, i: usize) -> Option<T> {
        if i >= self.data.len() || self.vacant.contains(&i) {
            // already vacant. nothing to do here
            return None;
        }
        // i is certainly within bounds of self.data
        let value = if self.reserved.remove(&i) {
            // no data to drop
            None
        } else {
            // invariant A => this element is OCCUPIED!
            unsafe {
                // 1. index is within bounds
                // 2. i is occupied => initialized data is being dropped
                Some(self.data.get_unchecked_mut(i).as_ptr().read())
            }
        };
        // Mark as vacant...
        if i + 1 == self.data.len() {
            // ... by truncating self.data.
            self.data.pop(); // truncate last data element
            let mut walking = i;
            while walking > 0 && self.vacant.remove(&(walking - 1)) {
                self.data.pop(); // truncate another element
                walking -= 1;
            }
        } else {
            // ... by populating self.vacant.
            self.vacant.insert(i);
        }
        value
    }
    pub fn iter_reserved(&self) -> impl Iterator<Item = usize> + '_ {
        self.reserved.iter().copied()
    }
}

fn pop_set_arb(s: &mut BTreeSet<usize>) -> Option<usize> {
    if let Some(&x) = s.iter().next() {
        s.remove(&x);
        Some(x)
    } else {
        None
    }
}

#[test]
fn vec_storage() {
    #[derive(Debug)]
    struct Foo;
    impl Drop for Foo {
        fn drop(&mut self) {
            println!("DROPPING FOO!");
        }
    }

    let mut v = VecStorage::default();
    let i0 = v.new_occupied(Foo);
    println!("{:?}", &v);
    let i1 = v.new_reserved();
    println!("{:?}", &v);
    let q = v.vacate(i0);
    println!("q {:?}", q);
    println!("{:?}", &v);
    v.occupy_reserved(i1, Foo);
    println!("{:?}", &v);
}
