extern crate test_generator;

use super::*;

use std::fs;
use std::path::Path;
use std::thread;
use test_generator::test_resources;

use crate::common::*;
use crate::runtime::*;

#[test_resources("testdata/connector/duo/*.apdl")]
fn batch1(resource: &str) {
    let a = Path::new(resource);
    let b = a.with_extension("bpdl");
    let a = fs::read_to_string(a).unwrap();
    let b = fs::read_to_string(b).unwrap();
    duo(a, b);
}

fn duo(one: String, two: String) {
    let a = thread::spawn(move || {
        let timeout = Duration::from_millis(1_500);
        let addrs = ["127.0.0.1:7010".parse().unwrap(), "127.0.0.1:7011".parse().unwrap()];
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
        x.configure(one.as_bytes()).unwrap();
        x.bind_port(0, PortBinding::Passive(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    let b = thread::spawn(move || {
        let timeout = Duration::from_millis(1_500);
        let addrs = ["127.0.0.1:7010".parse().unwrap(), "127.0.0.1:7011".parse().unwrap()];
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
        x.configure(two.as_bytes()).unwrap();
        x.bind_port(0, PortBinding::Passive(addrs[1])).unwrap();
        x.bind_port(1, PortBinding::Active(addrs[0])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    handle(a.join());
    handle(b.join());
}
