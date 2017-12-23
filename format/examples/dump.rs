extern crate casync_format;

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdin = stdin.lock();
    casync_format::dump_packets(io::BufReader::new(stdin)).unwrap();
}
