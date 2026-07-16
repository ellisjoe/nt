use crate::UdpMode::{Broadcast, Multicast, Unicast};
use chrono::Local;
use clap::{ArgGroup, Parser};
use std::io;
use std::io::{Read, Write, stdin};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};

const ALL_INTERFACES: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(allow_missing_positional = true)]
#[command(group(
    ArgGroup::new("protocol")
        .args(["tcp", "udp"])
        .multiple(false)
))]
struct Args {
    #[arg(short, long)]
    listen: bool,

    #[arg(short, long)]
    tcp: bool,

    #[arg(short, long)]
    udp: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(required_unless_present = "listen")]
    host: Option<String>,

    port: u16,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mode = get_mode(&args);

    let host = args.host.unwrap_or(ALL_INTERFACES.to_string());
    let port = args.port;

    if args.listen {
        let receiver = mode.get_receiver(host, port)?;

        let mut buf = [0; 1024];
        loop {
            let (num, addr) = receiver.read(&mut buf)?;
            let result = String::from_utf8_lossy(&buf[..num]);
            if args.verbose {
                println!(
                    "{} [{}] {}",
                    Local::now().format("%H:%M:%S%.3f"),
                    addr,
                    result.trim()
                );
            } else {
                println!("{}", result.trim());
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
                sent = sender.send(&bytes[sent..]).unwrap();
            }
        }
    }
}

fn get_mode(args: &Args) -> Protocol {
    if args.udp {
        Protocol::Udp
    } else {
        Protocol::Tcp
    }
}

enum Protocol {
    Udp,
    Tcp,
}

enum UdpMode {
    Unicast,
    Multicast(Ipv4Addr),
    Broadcast,
}

impl From<&str> for UdpMode {
    fn from(host: &str) -> Self {
        if let Ok(ip) = host.parse::<Ipv4Addr>() {
            if ip.is_multicast() {
                return Multicast(ip);
            } else if ip.is_broadcast() {
                return Broadcast;
            }
        }
        Unicast
    }
}

impl Protocol {
    fn get_sender(&self, host: String, port: u16) -> io::Result<Box<dyn Sender>> {
        match self {
            Protocol::Udp => {
                let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;

                match host.as_str().into() {
                    Unicast => {}
                    Multicast(ip) => udp.join_multicast_v4(&ip, &Ipv4Addr::UNSPECIFIED)?,
                    Broadcast => udp.set_broadcast(true)?,
                }

                udp.connect((host, port))?;
                Ok(Box::new(udp))
            }
            Protocol::Tcp => Ok(Box::new(TcpStream::connect((host, port))?)),
        }
    }

    fn get_receiver(&self, host: String, port: u16) -> io::Result<Box<dyn Receiver>> {
        match self {
            Protocol::Udp => {
                let mode: UdpMode = host.as_str().into();

                if let Multicast(ip) = mode {
                    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
                    udp.join_multicast_v4(&ip, &Ipv4Addr::UNSPECIFIED)?;
                    return Ok(Box::new(udp));
                }

                Ok(Box::new(UdpSocket::bind((host, port))?))
            }
            Protocol::Tcp => Ok(Box::new(TcpListener::bind((host, port))?.accept()?.0)),
        }
    }
}

trait Sender {
    fn send(&self, buf: &[u8]) -> io::Result<usize>;
}

trait Receiver {
    fn read(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
}

impl Sender for UdpSocket {
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        UdpSocket::send(self, buf)
    }
}

impl Sender for TcpStream {
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let mut sender = self;
        sender.write(buf)
    }
}

impl Receiver for UdpSocket {
    fn read(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.recv_from(buf)
    }
}

impl Receiver for TcpStream {
    fn read(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let mut receiver = self;
        Ok((Read::read(&mut receiver, buf)?, self.peer_addr()?))
    }
}
