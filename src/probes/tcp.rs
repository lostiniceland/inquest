use std::net::{TcpStream, ToSocketAddrs};

pub(crate) fn foo<A: ToSocketAddrs>(addr: A) {
    let x = TcpStream::connect(addr);

    match x {
        Ok(c) => println!("{:?}", c),
        Err(_) => println!("could not connect"),
    }
}
