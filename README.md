# sockgauge

A little TCP proxy that forwards all traffic as-is and reports the number of open sockets to the server. Written in Rust.

# Why?

`lsof` wasn't enough for debugging the pattern of leaky connections.