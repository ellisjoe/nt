#![deny(clippy::unwrap_used)]
pub mod error;

use crate::Protocol::{Tcp, Udp};
use crate::error::Result;
use chrono::Local;
use clap::{ArgGroup, Parser};
use socket2::{Domain, Socket, Type};
use std::io::{Read, Write, stdin, stdout};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};

const ALL_INTERFACES: &str = "0.0.0.0";

/// nt (nettool) is similar to nc (netcat) but with better support for udp
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(allow_missing_positional = true)]
#[command(group(
    ArgGroup::new("protocol")
        .args(["tcp", "udp"])
        .multiple(false)
))]
struct Args {
    /// Listen on the given port and prints output to the console
    #[arg(short, long)]
    listen: bool,

    /// Use a TCP socket for sending or receiving
    #[arg(short, long)]
    tcp: bool,

    /// Use a UDP socket for sending or receiving
    #[arg(short, long)]
    udp: bool,

    /// Output raw bytes rather than a utf8 string
    #[arg(short, long)]
    raw: bool,

    /// Print received messages in verbose mode with timestamps and source ip:port
    #[arg(short, long)]
    verbose: bool,

    /// The hostname or ip to connect to
    #[arg(required_unless_present = "listen")]
    host: Option<String>,

    /// The port to connect to
    port: u16,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let mode = if args.udp { Udp } else { Tcp };
    let host = args.host.as_deref().unwrap_or(ALL_INTERFACES);
    let port = args.port;

    if args.listen {
        let receiver = mode.get_receiver(host, port)?;

        let mut buf = [0; 1024];
        loop {
            let (num, addr) = receiver.read(&mut buf)?;
            if args.raw {
                stdout().write_all(&buf[..num])?;
                stdout().flush()?;
            } else {
                let string = String::from_utf8_lossy(&buf[..num]);
                let result = string.trim();
                if args.verbose {
                    let timestamp = Local::now().format("%H:%M:%S%.3f");
                    println!("{timestamp} [{addr}] {result}");
                } else {
                    println!("{result}");
                }
            }
        }
    } else {
        let sender = mode.get_sender(host, port)?;

        loop {
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            let bytes = input.as_bytes();

            let mut sent = 0;
            while sent < bytes.len() {
                sent = sender.send(&bytes[sent..])?;
            }
        }
    }
}

enum Protocol {
    Udp,
    Tcp,
}

impl Protocol {
    fn get_sender(&self, host: &str, port: u16) -> Result<Box<dyn Sender>> {
        match self {
            Udp => {
                let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;

                if host.parse::<Ipv4Addr>()?.is_broadcast() {
                    udp.set_broadcast(true)?;
                }

                udp.connect((host, port))?;
                Ok(Box::new(udp))
            }
            Tcp => Ok(Box::new(TcpStream::connect((host, port))?)),
        }
    }

    fn get_receiver(&self, host: &str, port: u16) -> Result<Box<dyn Receiver>> {
        match self {
            Protocol::Udp => {
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
            Protocol::Tcp => Ok(Box::new(TcpListener::bind((host, port))?.accept()?.0)),
        }
    }
}

trait Sender {
    fn send(&self, buf: &[u8]) -> Result<usize>;
}

trait Receiver {
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

#[cfg(test)]
mod tests {
    use super::*;
    use Protocol::Udp;
    use rstest::rstest;

    static MESSAGE: &str = "hello";
    static LOCALHOST: &str = "127.0.0.1";
    static MULTICAST: &str = "224.0.0.251";
    static BROADCAST: &str = "255.255.255.255";
    static PORT: u16 = 1111;

    #[test]
    fn test_udp_localhost() -> Result<()> {
        let receiver = Udp.get_receiver(LOCALHOST, PORT)?;
        let sender = Udp.get_sender(LOCALHOST, PORT)?;

        sender.send(MESSAGE.as_bytes())?;

        assert_eq!(receiver.read_string()?, MESSAGE);

        Ok(())
    }

    #[rstest]
    #[case::multicast(MULTICAST)]
    #[case::broadcast(BROADCAST)]
    fn test_udp_multi_receiver(#[case] host: &str) -> Result<()> {
        let receiver_1 = Udp.get_receiver(host, PORT)?;
        let receiver_2 = Udp.get_receiver(host, PORT)?;
        let sender = Udp.get_sender(host, PORT)?;

        sender.send(MESSAGE.as_bytes())?;

        assert_eq!(receiver_1.read_string()?, MESSAGE);
        assert_eq!(receiver_2.read_string()?, MESSAGE);

        Ok(())
    }

    trait ReadString {
        fn read_string(&self) -> Result<String>;
    }

    impl ReadString for dyn Receiver {
        fn read_string(&self) -> Result<String> {
            let mut buf = [0; 1024];
            let (n, _) = self.read(&mut buf)?;
            Ok(String::from_utf8_lossy(&buf[..n]).to_string())
        }
    }
}
