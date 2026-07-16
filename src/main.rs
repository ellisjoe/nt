use std::io;
use std::io::{stdin, Read, Write};
use clap::{ArgGroup, Parser};
use std::net::{TcpListener, TcpStream, UdpSocket};

const UDP_BROADCAST_ADDR: &str = "255.255.255.255";
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

    #[arg(required_unless_present = "listen")]
    host: Option<String>,

    port: i32,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mode = get_mode(&args);

    let host = args.host.unwrap_or(ALL_INTERFACES.to_string());
    let port = args.port;
    let address = format!("{}:{}", host, port);

    if args.listen {
        let receiver = mode.get_receiver(address)?;

        let mut buf = [0; 1024];
        loop {
            let num = receiver.read(&mut buf)?;
            let result = str::from_utf8(&buf[..num]).unwrap();
            println!("{}", result.trim());
        }
    } else {
        let sender = mode.get_sender(address)?;

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

impl Protocol {
    fn get_sender(&self, address: String) -> io::Result<Box<dyn Sender>> {
        match self {
            Protocol::Udp => {
                let udp = UdpSocket::bind(ALL_INTERFACES)?;
                if address.starts_with(UDP_BROADCAST_ADDR) {
                    udp.set_broadcast(true)?;
                }
                udp.connect(address)?;
                Ok(Box::new(udp))
            }
            Protocol::Tcp => {
                Ok(Box::new(TcpStream::connect(address)?))
            }
        }
    }

    fn get_receiver(&self, address: String) -> io::Result<Box<dyn Receiver>> {
        match self {
            Protocol::Udp => {
                Ok(Box::new(UdpSocket::bind(address)?))
            }
            Protocol::Tcp => {
                Ok(Box::new(TcpListener::bind(address)?.accept()?.0))
            }
        }
    }
}

trait Sender {
    fn send(&self, buf: &[u8]) -> io::Result<usize>;
}

trait Receiver {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
}

impl Sender for UdpSocket {
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        UdpSocket::send(self, buf)
    }
}

impl Sender for TcpStream {
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let mut sender = &*self;
        sender.write(buf)
    }
}

impl Receiver for UdpSocket {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_from(buf).map(|(n, _)| n)
    }
}

impl Receiver for TcpStream {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut receiver = &*self;
        Read::read(&mut receiver, buf)
    }
}