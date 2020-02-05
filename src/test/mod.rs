use crate::common::ControllerId;
use crate::runtime::Connector;
use crate::runtime::Unconfigured;
use core::fmt::Debug;

mod connector;
mod setup;

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
fn handle(result: Result<(), Box<(dyn std::any::Any + Send + 'static)>>) {
    if let Err(x) = result {
        panic!("Worker panicked: {:?}", Panicked(x))
    }
}

fn do_all(i: &[&(dyn Fn(&mut Connector) + Sync)]) {
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

    let mut failures = false;

    for ((controller_id, connector), res) in
        cid_iter.zip(connectors.iter_mut()).zip(results.into_iter())
    {
        println!("====================\n CID {:?} ...", controller_id);
        match connector.get_mut_logger() {
            Some(logger) => println!("{}", logger),
            None => println!("<No Log>"),
        }
        match res {
            Ok(()) => println!("CID {:?} OK!", controller_id),
            Err(e) => {
                failures = true;
                println!("CI {:?} PANIC! {:?}", controller_id, Panicked(e));
            }
        };
    }
    if failures {
        panic!("FAILURES!");
    }
}
