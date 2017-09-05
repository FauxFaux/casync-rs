extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate zstd;

mod chunks;
mod errors;
mod format;
mod index;
mod stream;

pub use errors::*;

pub use chunks::ChunkReader;
pub use index::read_index;
pub use stream::read_stream;
