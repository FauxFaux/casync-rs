extern crate byteorder;
#[macro_use]
extern crate error_chain;

mod errors;
mod format;
mod index;

pub use errors::*;

pub use index::read_index;
