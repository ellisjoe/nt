use crate::error::Result;
use socket2::{Domain, Socket, Type};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};

pub struct NtUdpSocket;

impl NtUdpSocket {
    pub fn create_sender(host: &str, port: u16) -> Result<Box<dyn Sender>> {
        let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;

        if let Ok(ip) = host.parse::<Ipv4Addr>() {
            udp.set_broadcast(ip.is_broadcast())?;
        }

        udp.connect((host, port))?;
        Ok(Box::new(udp))
    }

    pub fn create_receiver(host: &str, port: u16) -> Result<Box<dyn Receiver>> {
        let ip: Ipv4Addr = host.parse()?;

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(socket2::Protocol::UDP))?;

        let bind_addr: SocketAddr = if ip.is_multicast() || ip.is_broadcast() {
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            (Ipv4Addr::UNSPECIFIED, port).into()
        } else {
            (ip, port).into()
        };

        socket.bind(&bind_addr.into())?;

        if ip.is_multicast() {
            socket.join_multicast_v4(&ip, &Ipv4Addr::UNSPECIFIED)?;
        }

        let udp: UdpSocket = socket.into();
        Ok(Box::new(udp))
    }
}

pub trait Sender {
    fn send(&self, buf: &[u8]) -> Result<usize>;

    fn send_all(&self, buf: &[u8]) -> Result<()> {
        let mut sent = 0;
        while sent < buf.len() {
            sent = self.send(&buf[sent..])?;
        }
        Ok(())
    }
}

pub trait Receiver {
    fn read(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
}

impl Sender for UdpSocket {
    fn send(&self, buf: &[u8]) -> Result<usize> {
        Ok(UdpSocket::send(self, buf)?)
    }
}

impl Sender for TcpStream {
    fn send(&self, buf: &[u8]) -> Result<usize> {
        let mut sender = self;
        Ok(sender.write(buf)?)
    }
}

impl Receiver for UdpSocket {
    fn read(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        Ok(self.recv_from(buf)?)
    }
}

impl Receiver for TcpStream {
    fn read(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let mut receiver = self;
        Ok((Read::read(&mut receiver, buf)?, self.peer_addr()?))
    }
}
