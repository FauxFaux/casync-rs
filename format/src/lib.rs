extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate zstd;

mod errors;
mod format;
mod index;
mod chunks;

pub use errors::*;

pub use index::read_index;
pub use chunks::ChunkReader;
