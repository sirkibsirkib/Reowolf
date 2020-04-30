use mio::*;
use std::io::ErrorKind::WouldBlock;
use std::net::SocketAddr;

fn connection_testing_read(
    stream: &mut mio::net::TcpStream,
    inbox: &mut Vec<u8>,
) -> std::io::Result<()> {
    assert!(inbox.is_empty());
    use std::io::Read;
    match stream.read_to_end(inbox) {
        Ok(0) => unreachable!("Ok(0) on read should return Err instead!"),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}

#[test]
fn mio_tcp_connect_err() {
    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(64);

    let addr: SocketAddr = "127.0.0.1:12000".parse().unwrap();
    let mut stream = mio::net::TcpStream::connect(&addr).unwrap();
    poll.register(&stream, Token(0), Ready::all(), PollOpt::edge()).unwrap();

    let mut v = vec![];
    loop {
        poll.poll(&mut events, Some(std::time::Duration::from_secs(2))).unwrap();
        for event in events.iter() {
            assert_eq!(event.token(), Token(0));
            println!("readiness {:?}", event.readiness());
            // assert_eq!(event.readiness(), Ready::writable());

            v.clear();
            println!("{:?}", connection_testing_read(&mut stream, &mut v));
            println!("----------- {:?}", &v);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
