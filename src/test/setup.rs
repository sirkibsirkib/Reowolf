use super::*;
use crate::common::*;
use crate::runtime::*;
use PortBinding::*;

#[test]
fn config_ok_0() {
    let pdl = b"primitive main() {}";
    let d = ProtocolD::parse(pdl).unwrap();
    let pol = d.component_polarities(b"main").unwrap();
    assert_eq!(&pol[..], &[]);
}

#[test]
fn config_ok_2() {
    let pdl = b"primitive main(in x, out y) {}";
    let d = ProtocolD::parse(pdl).unwrap();
    let pol = d.component_polarities(b"main").unwrap();
    assert_eq!(&pol[..], &[Getter, Putter]);
}

#[test]
#[should_panic]
fn config_non_port() {
    let pdl = b"primitive main(in q, int q) {}";
    ProtocolD::parse(pdl).unwrap();
}

#[test]
fn bind_too_much() {
    let mut x = Connector::Unconfigured(Unconfigured { controller_id: 0 });
    x.configure(b"primitive main(in a) {}", b"main").unwrap();
    x.bind_port(0, Native).unwrap();
    assert!(x.bind_port(1, Native).is_err());
}

#[test]
fn config_and_connect_2() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr()];
    assert!(do_all(&[
        &|x| {
            x.configure(b"primitive main(in a, out b) {}", b"main").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Passive(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        },
        &|x| {
            x.configure(b"primitive main(out a, in b) {}", b"main").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.bind_port(1, Active(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        },
    ]));
}

#[test]
fn config_and_connect_chain() {
    let timeout = Duration::from_millis(1_500);
    let addrs = [next_addr(), next_addr(), next_addr()];
    assert!(do_all(&[
        &|x| {
            // PRODUCER A->
            x.configure(b"primitive main(out a) {}", b"main").unwrap();
            x.bind_port(0, Active(addrs[0])).unwrap();
            x.connect(timeout).unwrap();
        },
        &|x| {
            // FORWARDER ->B->
            x.configure(b"primitive main(in a, out b) {}", b"main").unwrap();
            x.bind_port(0, Passive(addrs[0])).unwrap();
            x.bind_port(1, Active(addrs[1])).unwrap();
            x.connect(timeout).unwrap();
        },
        &|x| {
            // FORWARDER ->C->
            x.configure(b"primitive main(in a, out b) {}", b"main").unwrap();
            x.bind_port(0, Passive(addrs[1])).unwrap();
            x.bind_port(1, Active(addrs[2])).unwrap();
            x.connect(timeout).unwrap();
        },
        &|x| {
            // CONSUMER ->D
            x.configure(b"primitive main(in a) {}", b"main").unwrap();
            x.bind_port(0, Passive(addrs[2])).unwrap();
            x.connect(timeout).unwrap();
        },
    ]));
}
