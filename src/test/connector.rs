extern crate test_generator;

use super::*;

use std::fs;
use std::path::Path;
use std::thread;
use test_generator::test_resources;

use crate::common::*;
use crate::runtime::*;
use crate::runtime::errors::*;

#[test]
fn incremental() {
    let timeout = Duration::from_millis(1_500);
    let addrs = ["127.0.0.1:7010".parse().unwrap(), "127.0.0.1:7011".parse().unwrap()];
    let a = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
        x.configure(
            b"primitive main(out a, out b) {
            synchronous {
                msg m = create(0);
                put(a, m);
            }
        }",
        )
        .unwrap();
        x.bind_port(0, PortBinding::Passive(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Passive(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    let b = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
        x.configure(
            b"primitive main(in a, in b) {
            synchronous {
                get(a);
            }
        }",
        )
        .unwrap();
        x.bind_port(0, PortBinding::Active(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    handle(a.join());
    handle(b.join());
}

#[test]
fn duo_positive() {
    let timeout = Duration::from_millis(1_500);
    let addrs = ["127.0.0.1:7012".parse().unwrap(), "127.0.0.1:7013".parse().unwrap()];
    let a = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
        x.configure(b"
        primitive main(out a, out b) {
            synchronous {}
            synchronous {}
            synchronous {
                msg m = create(0);
                put(a, m);
            }
            synchronous {
                msg m = create(0);
                put(b, m);
            }
        }").unwrap();
        x.bind_port(0, PortBinding::Passive(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Passive(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    let b = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
        x.configure(b"
        primitive main(in a, in b) {
            while (true) {
                synchronous {
                    if (fires(a)) {
                        get(a);
                    }
                }
                synchronous {
                    if (fires(b)) {
                        get(b);
                    }
                }
            }
        }").unwrap();
        x.bind_port(0, PortBinding::Active(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
    });
    handle(a.join());
    handle(b.join());
}

#[test]
fn duo_negative() {
    let timeout = Duration::from_millis(500);
    let addrs = ["127.0.0.1:7012".parse().unwrap(), "127.0.0.1:7013".parse().unwrap()];
    let a = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
        x.configure(b"
        primitive main(out a, out b) {
            synchronous {}
            synchronous {
                msg m = create(0);
                put(a, m); // fires a on second round
            }
        }").unwrap();
        x.bind_port(0, PortBinding::Passive(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Passive(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        match x.sync(timeout) {
            Err(SyncErr::Timeout) => {}
            x => unreachable!("{:?}", x)
        }
    });
    let b = thread::spawn(move || {
        let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
        x.configure(b"
        primitive main(in a, in b) {
            while (true) {
                synchronous {
                    if (fires(a)) {
                        get(a);
                    }
                }
                synchronous {
                    if (fires(b)) { // never fire a on even round
                        get(b);
                    }
                }
            }
        }").unwrap();
        x.bind_port(0, PortBinding::Active(addrs[0])).unwrap();
        x.bind_port(1, PortBinding::Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        match x.sync(timeout) {
            Err(SyncErr::Timeout) => {}
            x => unreachable!("{:?}", x)
        }
    });
    handle(a.join());
    handle(b.join());
}
