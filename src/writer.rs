use chrono::Local;
use std::borrow::Cow;
use std::net::SocketAddr;

pub trait Formatter {
    fn format<'a>(&self, addr: SocketAddr, buf: &'a [u8]) -> Cow<'a, [u8]>;
}

pub struct RawFormatter;

impl Formatter for RawFormatter {
    fn format<'a>(&self, _addr: SocketAddr, buf: &'a [u8]) -> Cow<'a, [u8]> {
        Cow::Borrowed(buf)
    }
}

pub struct VerboseFormatter;

impl Formatter for VerboseFormatter {
    fn format<'a>(&self, addr: SocketAddr, buf: &'a [u8]) -> Cow<'a, [u8]> {
        let timestamp = Local::now().format("%H:%M:%S%.3f");
        let result = format!("{timestamp} [{addr}] {}", String::from_utf8_lossy(buf));
        Cow::Owned(result.into_bytes())
    }
}
