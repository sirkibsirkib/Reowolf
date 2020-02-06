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
                b"main",
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
                b"main",
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
            b"main",
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
            b"main",
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
            b"main",
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
            b"main",
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

static FORWARD: &[u8] = b"
primitive forward(in i, out o) {
    while(true) synchronous {
        put(o, get(i));
    }
}";

#[test]
fn connect_natives() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    do_all(&[
        &|x| {
            x.configure(FORWARD, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
        &|x| {
            x.configure(FORWARD, b"forward").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
    ]);
}

#[test]
fn forward() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    do_all(&[
        //
        &|x| {
            x.configure(FORWARD, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();

            let msg = b"HELLO!".to_vec();
            x.put(0, msg).unwrap();
            assert_eq!(0, x.sync(timeout).unwrap());
        },
        &|x| {
            x.configure(FORWARD, b"forward").unwrap();
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

static SYNC: &[u8] = b"
primitive sync(in i, out o) {
    while(true) synchronous {
        if (fires(i)) put(o, get(i));
    }
}";
#[test]
fn native_alt() {
    /*
    Alice -->sync--A|P-->sync--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 3;
    do_all(&[
        //
        &|x| {
            x.configure(SYNC, b"sync").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();

            let msg = b"HI".to_vec();
            for _i in 0..N {
                // round _i*2: batches: [0=>*]
                assert_eq!(0, x.sync(timeout).unwrap());

                // round _i*2+1: batches: [0=>HI]
                x.put(0, msg.clone()).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
            }
        },
        &|x| {
            x.configure(SYNC, b"sync").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            let expect = b"HI".to_vec();
            for _i in 0..(2 * N) {
                // round _i batches:[0=>*, 0=>HI]
                x.next_batch().unwrap();
                x.get(0).unwrap();
                match x.sync(timeout).unwrap() {
                    0 => assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0)),
                    1 => assert_eq!(Ok(&expect[..]), x.read_gotten(0)),
                    _ => unreachable!(),
                }
            }
        },
    ]);
}

static ALTERNATOR_2: &[u8] = b"
primitive alternator_2(in i, out a, out b) {
    while(true) {
        synchronous { put(a, get(i)); }
        synchronous { put(b, get(i)); } 
    }
}";

#[test]
fn alternator_2() {
    /*                    /--|-->A
    Sender -->alternator_2
                          \--|-->B
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 5;
    do_all(&[
        //
        &|x| {
            // Sender
            x.configure(ALTERNATOR_2, b"alternator_2").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.bind_port(2, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                for _ in 0..2 {
                    x.put(0, b"hey".to_vec()).unwrap();
                    assert_eq!(0, x.sync(timeout).unwrap());
                }
            }
        },
        &|x| {
            // A
            x.configure(SYNC, b"sync").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            let expecting: &[u8] = b"hey";

            for _ in 0..N {
                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(expecting), x.read_gotten(0));

                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));
            }
        },
        &|x| {
            // B
            x.configure(SYNC, b"sync").unwrap();
            x.bind_port(0, Active(addrs[1])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            let expecting: &[u8] = b"hey";

            for _ in 0..N {
                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));

                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(expecting), x.read_gotten(0));
            }
        },
    ]);
}

static CHAIN: &[u8] = b"
primitive sync(in i, out o) {
    while(true) synchronous {
        if (fires(i)) put(o, get(i));
    }
}
composite sync_2(in i, out o) {
    channel x -> y;
    new sync(i, x);
    new sync(y, o);
}";

#[test]
fn composite_chain() {
    /*
    Alice -->sync-->sync-->A|P-->sync-->sync--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 1;
    static MSG: &[u8] = b"Hi, there.";
    do_all(&[
        //
        &|x| {
            // Alice
            x.configure(CHAIN, b"sync_2").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.put(0, MSG.to_vec()).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
            }
        },
        &|x| {
            // Bob
            x.configure(CHAIN, b"sync_2").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]);
}

static PARITY_ROUTER: &[u8] = b"
primitive parity_router(in i, out odd, out even) {
    while(true) synchronous {
        msg m = get(i);
        if (m[0]%2==0) {
            put(even, m);
        } else {
            put(odd, m);
        }
    }
}";

#[test]
// THIS DOES NOT YET WORK. TODOS are hit
fn parity_router() {
    /*                    /--|-->Getsodd
    Sender -->parity_router
                          \--|-->Getseven
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 1;
    do_all(&[
        //
        &|x| {
            // Sender
            x.configure(PARITY_ROUTER, b"parity_router").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.bind_port(2, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();

            for i in 0..N {
                let msg = vec![i as u8]; // messages [0], [1], [2], ...
                x.put(0, msg).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
            }
        },
        &|x| {
            // Getsodd
            x.configure(FORWARD, b"forward").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                // round _i batches:[0=>*, 0=>?]
                x.next_batch().unwrap();
                x.get(0).unwrap();
                match x.sync(timeout).unwrap() {
                    0 => assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0)),
                    1 => {
                        let msg = x.read_gotten(0).unwrap();
                        assert!(msg[0] % 2 == 1); // assert msg is odd
                    }
                    _ => unreachable!(),
                }
            }
        },
        &|x| {
            // Getseven
            x.configure(FORWARD, b"forward").unwrap();
            x.bind_port(0, Active(addrs[1])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                // round _i batches:[0=>*, 0=>?]
                x.next_batch().unwrap();
                x.get(0).unwrap();
                match x.sync(timeout).unwrap() {
                    0 => assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0)),
                    1 => {
                        let msg = x.read_gotten(0).unwrap();
                        assert!(msg[0] % 2 == 0); // assert msg is even
                    }
                    _ => unreachable!(),
                }
            }
        },
    ]);
}
