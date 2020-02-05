extern crate test_generator;

use super::*;

use std::thread;

use crate::common::*;
use crate::runtime::{errors::*, PortBinding::*, *};

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

#[test]
fn incremental() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    let handles = vec![
        thread::spawn(move || {
            let controller_id = 0;
            let mut x = Connector::Unconfigured(Unconfigured { controller_id });
            x.configure(
                b"primitive main(out a, out b) {
                    synchronous {
                        msg m = create(0);
                        put(a, m);
                    }
                }",
            )
            .unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
            println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
        }),
        thread::spawn(move || {
            let controller_id = 1;
            let mut x = Connector::Unconfigured(Unconfigured { controller_id });
            x.configure(
                b"primitive main(in a, in b) {
                    synchronous {
                        get(a);
                    }
                }",
            )
            .unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Active(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
            println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
        }),
    ];
    for h in handles {
        handle(h.join())
    }
}

#[test]
fn duo_positive() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    let a = thread::spawn(move || {
        let controller_id = 0;
        let mut x = Connector::Unconfigured(Unconfigured { controller_id });
        x.configure(
            b"primitive main(out a, out b) {
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
            }",
        )
        .unwrap();
        x.bind_port(0, Passive(addrs[0])).unwrap();
        x.bind_port(1, Passive(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
    });
    let b = thread::spawn(move || {
        let controller_id = 1;
        let mut x = Connector::Unconfigured(Unconfigured { controller_id });
        x.configure(
            b"primitive main(in a, in b) {
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
            }",
        )
        .unwrap();
        x.bind_port(0, Active(addrs[0])).unwrap();
        x.bind_port(1, Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        assert_eq!(0, x.sync(timeout).unwrap());
        println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
    });
    handle(a.join());
    handle(b.join());
}

#[test]
fn duo_negative() {
    let timeout = Duration::from_millis(500);
    let addrs = [next_addr(), next_addr()];
    let a = thread::spawn(move || {
        let controller_id = 0;
        let mut x = Connector::Unconfigured(Unconfigured { controller_id });
        x.configure(
            b"primitive main(out a, out b) {
                synchronous {}
                synchronous {
                    msg m = create(0);
                    put(a, m); // fires a on second round
                }
            }",
        )
        .unwrap();
        x.bind_port(0, Passive(addrs[0])).unwrap();
        x.bind_port(1, Passive(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        let r = x.sync(timeout);
        println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
        match r {
            Err(SyncErr::Timeout) => {}
            x => unreachable!("{:?}", x),
        }
    });
    let b = thread::spawn(move || {
        let controller_id = 1;
        let mut x = Connector::Unconfigured(Unconfigured { controller_id });
        x.configure(
            b"primitive main(in a, in b) {
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
            }",
        )
        .unwrap();
        x.bind_port(0, Active(addrs[0])).unwrap();
        x.bind_port(1, Active(addrs[1])).unwrap();
        x.connect(timeout).unwrap();
        assert_eq!(0, x.sync(timeout).unwrap());
        let r = x.sync(timeout);
        println!("\n---------\nLOG CID={}\n{}", controller_id, x.get_mut_logger().unwrap());
        match r {
            Err(SyncErr::Timeout) => {}
            x => unreachable!("{:?}", x),
        }
    });
    handle(a.join());
    handle(b.join());
}

#[test]
fn connect_natives() {
    static CHAIN: &[u8] = b"
    primitive main(in i, out o) {
        while(true) synchronous {}
    }";
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    do_all(&[
        &|x| {
            x.configure(CHAIN).unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
        &|x| {
            x.configure(CHAIN).unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
    ]);
}

#[test]
fn forward() {
    static FORWARD: &[u8] = b"
    primitive main(in i, out o) {
        while(true) synchronous {
            put(o, get(i));
        }
    }";
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    do_all(&[
        //
        &|x| {
            x.configure(FORWARD).unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();

            let msg = b"HELLO!".to_vec();
            x.put(0, msg).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
        &|x| {
            x.configure(FORWARD).unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            let expect = b"HELLO!".to_vec();
            x.get(0).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
            assert_eq!(expect, x.read_gotten(0).unwrap());
        },
    ]);
}
