[![CI](https://github.com/ellisjoe/nt/actions/workflows/rust.yml/badge.svg)](https://github.com/ellisjoe/nt/actions/workflows/rust.yml/badge.svg)

NetTool
=======

```
nt (nettool) is similar to nc (netcat) but with better support for udp

Usage: nt [OPTIONS] [HOST] <PORT>

Arguments:
  [HOST]  The hostname or ip to connect to
  <PORT>  The port to connect to

Options:
  -l, --listen   Listen on the given port and prints output to the console
  -t, --tcp      Use a TCP socket for sending or receiving [default]
  -u, --udp      Use a UDP socket for sending or receiving
  -r, --raw      Output raw bytes rather than a utf8 string
  -v, --verbose  Print received messages in verbose mode with timestamps and source ip:port
  -h, --help     Print help
  -V, --version  Print version
```
