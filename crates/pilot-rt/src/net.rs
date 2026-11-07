//! Raw OS TCP/UDP Socket Syscalls (`stdlib/net`).
//! Declarative HTTP/1.1 client logic lives in `stdlib/http/http.pilot`.

use std::io;
use std::net::{TcpListener, TcpStream, UdpSocket};

pub fn tcp_connect(addr: &str) -> io::Result<TcpStream> {
    TcpStream::connect(addr)
}

pub fn tcp_listen(addr: &str) -> io::Result<TcpListener> {
    TcpListener::bind(addr)
}

pub fn udp_bind(addr: &str) -> io::Result<UdpSocket> {
    UdpSocket::bind(addr)
}
