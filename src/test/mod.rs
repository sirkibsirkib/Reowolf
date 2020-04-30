use crate::common::ControllerId;
use crate::runtime::Connector;
use crate::runtime::Unconfigured;
use core::fmt::Debug;
use std::net::SocketAddr;

mod connector;
mod setup;

// using a static AtomicU16, shared between all tests in the binary,
// allocate and return a socketaddr of the form 127.0.0.1:X where X in 7000..
fn next_addr() -> SocketAddr {
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        sync::atomic::{AtomicU16, Ordering::SeqCst},
    };
    static TEST_PORT: AtomicU16 = AtomicU16::new(7_000);
    let port = TEST_PORT.fetch_add(1, SeqCst);
    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port).into()
}

struct Panicked(Box<dyn std::any::Any>);
impl Debug for Panicked {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(str_slice) = self.0.downcast_ref::<&'static str>() {
            f.pad(str_slice)
        } else if let Some(string) = self.0.downcast_ref::<String>() {
            f.pad(string)
        } else {
            f.pad("Box<Any>")
        }
    }
}

// Given a set of tasks (where each is some function that interacts with a connector)
// run each task in in its own thread.
// print the log and OK/PANIC result of each thread
// then finally, return true IFF no threads panicked
fn run_connector_set(i: &[&(dyn Fn(&mut Connector) + Sync)]) -> bool {
    let cid_iter = 0..(i.len() as ControllerId);
    let mut connectors = cid_iter
        .clone()
        .map(|controller_id| Connector::Unconfigured(Unconfigured { controller_id }))
        .collect::<Vec<_>>();

    let mut results = vec![];
    crossbeam_utils::thread::scope(|s| {
        let handles: Vec<_> = i
            .iter()
            .zip(connectors.iter_mut())
            .map(|(func, connector)| s.spawn(move |_| func(connector)))
            .collect();
        for h in handles {
            results.push(h.join());
        }
    })
    .unwrap();

    let mut alright = true;

    for ((controller_id, connector), res) in
        cid_iter.zip(connectors.iter_mut()).zip(results.into_iter())
    {
        println!("\n\n====================\n CID {:?} ...", controller_id);
        match connector.get_mut_logger() {
            Some(logger) => println!("{}", logger),
            None => println!("<No Log>"),
        }
        match res {
            Ok(()) => println!("CID {:?} OK!", controller_id),
            Err(e) => {
                alright = false;
                println!("CI {:?} PANIC! {:?}", controller_id, Panicked(e));
            }
        };
    }
    alright
}
