extern crate test_generator;

use super::*;

use crate::common::*;
use crate::runtime::{errors::*, PortBinding::*};

static PDL: &[u8] = b"
primitive blocked(in i, out o) {
    while(true) synchronous {}
}
primitive forward(in i, out o) {
    while(true) synchronous {
        put(o, get(i));
    }
}
primitive sync(in i, out o) {
    while(true) synchronous {
        if (fires(i)) put(o, get(i));
    }
}
primitive fifo_1(in i, out o) {
    msg holding = null;
    while(true) synchronous {
        if (holding == null && fires(i)) {
            holding = get(i);
        } else if (holding != null && fires(o)) {
            put(o, holding);
            holding = null;
        }
    }
}
primitive alternator_2(in i, out a, out b) {
    while(true) {
        synchronous { put(a, get(i)); }
        synchronous { put(b, get(i)); } 
    }
}
composite sync_2(in i, out o) {
    channel x -> y;
    new sync(i, x);
    new sync(y, o);
}
composite forward_pair(in ia, out oa, in ib, out ob) {
    new forward(ia, oa);
    new forward(ib, ob);
}
primitive forward_nonzero(in i, out o) {
    while(true) synchronous {
        msg m = get(i);
        assert(m[0] != 0);
        put(o, m);
    }
}
primitive token_spout(out o) {
    while(true) synchronous {
        put(o, create(0));
    }
}
primitive wait_n(int to_wait, out o) {
    while(to_wait > 0) synchronous() to_wait -= 1;
    synchronous { put(o, create(0)); }
}
composite wait_10(out o) {
    new wait_n(10, o);
}
";

#[test]
fn connects_ok() {
    // Test if we can connect natives using the given PDL
    /*
    Alice -->silence--P|A-->silence--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    assert!(run_connector_set(&[
        &|x| {
            // Alice
            x.configure(PDL, b"blocked").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
        },
        &|x| {
            // Bob
            x.configure(PDL, b"blocked").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
        },
    ]));
}

#[test]
fn connected_but_silent_natives() {
    // Test if we can connect natives and have a trivial sync round
    /*
    Alice -->silence--P|A-->silence--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    assert!(run_connector_set(&[
        &|x| {
            // Alice
            x.configure(PDL, b"blocked").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(Ok(0), x.sync(timeout));
        },
        &|x| {
            // Bob
            x.configure(PDL, b"blocked").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            assert_eq!(Ok(0), x.sync(timeout));
        },
    ]));
}

#[test]
fn self_forward_ok() {
    // Test a deterministic system
    // where a native has no network bindings
    // and sends messages to itself
    /*
        /-->\
    Alice   forward
        \<--/
    */
    let timeout = Duration::from_millis(1_500);
    const N: usize = 5;
    static MSG: &[u8] = b"Echo!";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.put(0, MSG.to_vec()).unwrap();
                x.get(1).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(1));
            }
        },
    ]));
}
#[test]
fn token_spout_ok() {
    // Test a deterministic system where the proto
    // creates token messages
    /*
    Alice<--token_spout
    */
    let timeout = Duration::from_millis(1_500);
    const N: usize = 5;
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"token_spout").unwrap();
            x.bind_port(0, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(&[] as &[u8]), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn waiter_ok() {
    // Test a stateful proto that blocks port 0 for 10 rounds
    // and then sends a single token on the 11th
    /*
    Alice<--token_spout
    */
    let timeout = Duration::from_millis(1_500);
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"wait_10").unwrap();
            x.bind_port(0, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..10 {
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));
            }
            x.get(0).unwrap();
            assert_eq!(Ok(0), x.sync(timeout));
            assert_eq!(Ok(&[] as &[u8]), x.read_gotten(0));
        },
    ]));
}

#[test]
fn self_forward_timeout() {
    // Test a deterministic system
    // where a native has no network bindings
    // and sends messages to itself
    /*
        /-->\
    Alice   forward
        \<--/
    */
    let timeout = Duration::from_millis(500);
    static MSG: &[u8] = b"Echo!";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Sender
            x.configure(PDL, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            x.put(0, MSG.to_vec()).unwrap();
            // native and forward components cannot find a solution
            assert_eq!(Err(SyncErr::Timeout), x.sync(timeout));
        },
    ]));
}

#[test]
fn forward_det() {
    // Test if a deterministic protocol and natives can pass one message
    /*
    Alice -->forward--P|A-->forward--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 5;
    static MSG: &[u8] = b"Hello!";

    assert!(run_connector_set(&[
        &|x| {
            x.configure(PDL, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.put(0, MSG.to_vec()).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
            }
        },
        &|x| {
            x.configure(PDL, b"forward").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn nondet_proto_det_natives() {
    // Test the use of a nondeterministic protocol
    // where Alice decides the choice and the others conform
    /*
    Alice -->sync--A|P-->sync--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 5;
    static MSG: &[u8] = b"Message, here!";
    assert!(run_connector_set(&[
        &|x| {
            // Alice
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            for _i in 0..N {
                x.put(0, MSG.to_vec()).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
            }
        },
        &|x| {
            // Bob
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _i in 0..N {
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn putter_determines() {
    // putter and getter
    /*
    Alice -->sync--A|P-->sync--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 3;
    static MSG: &[u8] = b"Hidey ho!";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            for _i in 0..N {
                x.put(0, MSG.to_vec()).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
            }
        },
        &|x| {
            // Bob
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _i in 0..N {
                // batches [{0=>*}, {0=>?}]
                x.get(0).unwrap();
                x.next_batch().unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn getter_determines() {
    // putter and getter
    /*
    Alice -->sync--A|P-->sync--> Bob
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 5;
    static MSG: &[u8] = b"Hidey ho!";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
            for _i in 0..N {
                // batches [{0=>?}, {0=>*}]
                x.put(0, MSG.to_vec()).unwrap();
                x.next_batch().unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
            }
        },
        &|x| {
            // Bob
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            for _i in 0..N {
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn fifo_2() {
    // Test a deterministic system which
    // alternates sending Sender's messages to A or B
    /*                    /--|-->A
    Sender -->alternator_2
                          \--|-->B
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 5;
    static MSG: &[u8] = b"message";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Sender
            x.configure(PDL, b"alternator_2").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.bind_port(2, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                for _ in 0..2 {
                    x.put(0, MSG.to_vec()).unwrap();
                    assert_eq!(0, x.sync(timeout).unwrap());
                }
            }
        },
        &|x| {
            // A
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(MSG), x.read_gotten(0));

                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));
            }
        },
        &|x| {
            // B
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Active(addrs[1])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));

                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
fn alternator_2() {
    // Test a deterministic system which
    // alternates sending Sender's messages to A or B
    /*                    /--|-->A
    Sender -->alternator_2
                          \--|-->B
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 5;
    static MSG: &[u8] = b"message";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Sender
            x.configure(PDL, b"alternator_2").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.bind_port(2, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                for _ in 0..2 {
                    x.put(0, MSG.to_vec()).unwrap();
                    assert_eq!(0, x.sync(timeout).unwrap());
                }
            }
        },
        &|x| {
            // A
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(MSG), x.read_gotten(0));

                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));
            }
        },
        &|x| {
            // B
            x.configure(PDL, b"sync").unwrap();
            x.bind_port(0, Active(addrs[1])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();

            for _ in 0..N {
                // silent round
                assert_eq!(Ok(0), x.sync(timeout)); // MISS ONE
                assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));

                // get msg round
                x.get(0).unwrap();
                assert_eq!(Ok(0), x.sync(timeout)); // GET ONE
                assert_eq!(Ok(MSG), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
// PANIC TODO: eval::1536
fn composite_chain() {
    // Check if composition works. Forward messages through long chains
    /*
    Alice -->sync-->sync-->A|P-->sync-->sync--> Bob
    */
    static PDL : &[u8] =
b"primitive sync(in i, out o) {
    while(true) synchronous {
        if (fires(i)) put(o, get(i));
    }
}
composite sync_2(in i, out o) {
    channel x -> y;
    new sync(i, x);
    new sync(y, o);
}";
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 1;
    static MSG: &[u8] = b"SSS";
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"sync_2").unwrap();
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
            x.configure(PDL, b"sync").unwrap();
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
    ]));
}

#[test]
// PANIC TODO: eval::1605
fn exchange() {
    /*
        /-->forward-->P|A-->forward-->\
    Alice                             Bob
        \<--forward<--P|A<--forward<--/
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    const N: usize = 1;
    assert!(run_connector_set(&[
        //
        &|x| {
            // Alice
            x.configure(PDL, b"forward_pair").unwrap();
            x.bind_port(0, Native).unwrap(); // native in
            x.bind_port(1, Passive(addrs[0])).unwrap(); // peer out
            x.bind_port(2, Passive(addrs[1])).unwrap(); // peer in
            x.bind_port(3, Native).unwrap(); // native out
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.put(0, b"A->B".to_vec()).unwrap();
                x.get(1).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(b"B->A" as &[u8]), x.read_gotten(0));
            }
        },
        &|x| {
            // Bob
            x.configure(PDL, b"forward_pair").unwrap();
            x.bind_port(0, Native).unwrap(); // native in
            x.bind_port(1, Active(addrs[1])).unwrap(); // peer out
            x.bind_port(2, Active(addrs[0])).unwrap(); // peer in
            x.bind_port(3, Native).unwrap(); // native out
            x.connect(timeout).unwrap();
            for _ in 0..N {
                x.put(0, b"B->A".to_vec()).unwrap();
                x.get(1).unwrap();
                assert_eq!(Ok(0), x.sync(timeout));
                assert_eq!(Ok(b"A->B" as &[u8]), x.read_gotten(0));
            }
        },
    ]));
}

#[test]
// THIS DOES NOT YET WORK. TODOS are hit
fn filter_messages() {
    // Make a protocol whose behavior depends on the contents of messages
    // Getter prohibits the receipt of messages of the form [0, ...].
    // those messages are silent
    /*
    Sender -->forward-->P|A-->forward_nonzero--> Receiver
    */
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr()];
    const N: usize = 1;
    assert!(run_connector_set(&[
        //
        &|x| {
            // Sender
            x.configure(PDL, b"forward").unwrap();
            x.bind_port(0, Native).unwrap();
            x.bind_port(1, Passive(addrs[0])).unwrap();
            x.connect(timeout).unwrap();

            for i in (0..3).cycle().take(N) {
                let msg = vec![i as u8]; // messages [0], [1], [2], [0], [1] ...
                                         // batches: [{0=>*}, {0=>?}]
                x.next_batch().unwrap();
                x.put(0, msg.clone()).unwrap();
                assert_eq!(0, x.sync(timeout).unwrap());
                match x.sync(timeout).unwrap() {
                    0 => {
                        // not sent
                        assert_eq!(&msg, &[0u8]);
                    }
                    1 => {
                        // sent
                        assert_ne!(&msg, &[0u8]);
                    }
                    _ => unreachable!(),
                }
            }
        },
        &|x| {
            // Receiver
            x.configure(PDL, b"forward_nonzero").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Native).unwrap();
            x.connect(timeout).unwrap();
            for _ in 0..N {
                // round _i batches:[0=>*, 0=>?]
                x.next_batch().unwrap();
                x.get(0).unwrap();
                match x.sync(timeout).unwrap() {
                    0 => {
                        // nothing received
                        assert_eq!(Err(ReadGottenErr::DidNotGet), x.read_gotten(0));
                    }
                    1 => {
                        // msg received
                        assert_ne!(&[0u8], x.read_gotten(0).unwrap());
                    }
                    _ => unreachable!(),
                }
            }
        },
    ]));
}
