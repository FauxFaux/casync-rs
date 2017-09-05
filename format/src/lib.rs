extern crate byteorder;
#[macro_use]
extern crate error_chain;

mod errors;
mod format;
mod index;
mod stream;

pub use errors::*;

pub use index::read_index;
pub use stream::ChunkReader;
