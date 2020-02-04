use crate::common::*;
use crate::runtime::*;

use PortBinding::*;

use super::*;

#[test]
fn config_ok_0() {
    let pdl = b"primitive main() {}";
    let d = ProtocolD::parse(pdl).unwrap();
    let pol = d.main_interface_polarities();
    assert_eq!(&pol[..], &[]);
}

#[test]
fn config_ok_2() {
    let pdl = b"primitive main(in x, out y) {}";
    let d = ProtocolD::parse(pdl).unwrap();
    let pol = d.main_interface_polarities();
    assert_eq!(&pol[..], &[Polarity::Getter, Polarity::Putter]);
}

#[test]
#[should_panic]
fn config_non_port() {
    let pdl = b"primitive main(in q, int q) {}";
    ProtocolD::parse(pdl).unwrap();
}

#[test]
fn config_and_connect_2() {
    let timeout = Duration::from_millis(1_500);
    let addrs = ["127.0.0.1:9000".parse().unwrap(), "127.0.0.1:9001".parse().unwrap()];
    use std::thread;
    let handles = vec![
        //
        thread::spawn(move || {
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
            x.configure(b"primitive main(in a, out b) {}").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        }),
        thread::spawn(move || {
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
            x.configure(b"primitive main(out a, in b) {}").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Active(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        }),
    ];
    for h in handles {
        handle(h.join())
    }
}

#[test]
fn bind_too_much() {
    let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
    x.configure(b"primitive main(in a) {}").unwrap();
    x.bind_port(0, Native).unwrap();
    assert!(x.bind_port(1, Native).is_err());
}

#[test]
fn config_and_connect_chain() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [
        "127.0.0.1:9002".parse().unwrap(),
        "127.0.0.1:9003".parse().unwrap(),
        "127.0.0.1:9004".parse().unwrap(),
    ];
    use std::thread;
    let handles = vec![
        //
        thread::spawn(move || {
            // PRODUCER A->
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
            x.configure(b"primitive main(out a) {}").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
        }),
        thread::spawn(move || {
            // FORWARDER ->B->
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 1 });
            x.configure(b"primitive main(in a, out b) {}").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Active(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        }),
        thread::spawn(move || {
            // FORWARDER ->C->
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 2 });
            x.configure(b"primitive main(in a, out b) {}").unwrap();
            x.bind_port(0, Passive(addrs[1])).unwrap();
            x.bind_port(1, Active(addrs[2])).unwrap();
            x.connect(timeout).unwrap();
        }),
        thread::spawn(move || {
            // CONSUMER ->D
            let mut x = Connector::Unconfigured(Unconfigured { controller_id: 3 });
            x.configure(b"primitive main(in a) {}").unwrap();
            x.bind_port(0, Passive(addrs[2])).unwrap();
            x.connect(timeout).unwrap();
        }),
    ];
    for h in handles {
        handle(h.join())
    }
}
