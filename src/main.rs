use crate::UdpMode::{Broadcast, Multicast, Unicast};
use chrono::Local;
use clap::{ArgGroup, Parser};
use socket2::{Domain, Socket, Type};
use std::io;
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
            if args.raw {
                stdout().write_all(&buf[..num])?;
                stdout().flush()?;
            } else {
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

                if matches!(host.as_str().into(), Broadcast) {
                    udp.set_broadcast(true)?;
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

                let bind_addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
                let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(socket2::Protocol::UDP))?;

                if matches!(mode, Broadcast | Multicast(_)) {
                    socket.set_reuse_address(true)?;
                    socket.set_reuse_port(true)?;
                }

                socket.bind(&bind_addr.into())?;

                if let Multicast(ip) = mode {
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
