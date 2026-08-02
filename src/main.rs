#![deny(clippy::unwrap_used)]
pub mod error;
mod formatter;
mod socket;

use crate::Protocol::{Tcp, Udp};
use crate::error::Result;
use crate::formatter::{DefaultFormatter, Formatter, RawFormatter, VerboseFormatter};
use crate::socket::{NtUdpSocket, Receiver, Sender};
use clap::{ArgGroup, Parser};
use std::io::{Write, stdin, stdout};
use std::net::{TcpListener, TcpStream};

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

    /// Use a TCP socket for sending or receiving [default]
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

    let formatter: &dyn Formatter = if args.raw {
        &RawFormatter
    } else if args.verbose {
        &VerboseFormatter
    } else {
        &DefaultFormatter
    };

    if args.listen {
        let receiver = mode.get_receiver(host, port)?;

        let mut buf = [0; 1024];
        loop {
            let (num, addr) = receiver.read(&mut buf)?;
            let formatted = formatter.format(addr, &buf[..num]);
            stdout().write_all(formatted.as_ref())?;
            stdout().flush()?;
        }
    } else {
        let sender = mode.get_sender(host, port)?;

        loop {
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            sender.send_all(input.as_bytes())?;
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
            Udp => NtUdpSocket::create_sender(host, port),
            Tcp => Ok(Box::new(TcpStream::connect((host, port))?)),
        }
    }

    fn get_receiver(&self, host: &str, port: u16) -> Result<Box<dyn Receiver>> {
        match self {
            Udp => NtUdpSocket::create_receiver(host, port),
            Tcp => Ok(Box::new(TcpListener::bind((host, port))?.accept()?.0)),
        }
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
