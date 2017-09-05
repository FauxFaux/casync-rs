extern crate casync_format;

use std::io;

#[test]
fn load_index() {
    let file = io::Cursor::new(&include_bytes!("data/trivial.caidx")[..]);
    casync_format::read_index(file).unwrap();
}
