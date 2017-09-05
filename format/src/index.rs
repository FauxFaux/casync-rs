use std;
use std::fmt;
use std::io;
use std::io::Read;

use errors::*;
use format;

use byteorder::{ReadBytesExt, LittleEndian};
use hex_slice::AsHex;

struct ChunkSize {
    min: u64,
    avg: u64,
    max: u64,
}

impl ChunkSize {
    fn new(min: u64, avg: u64, max: u64) -> Result<ChunkSize> {
        ensure!(min >= 1, "minimum chunk size is too low");
        ensure!(max <= 128 * 1024 * 1024, "maximum chunk size is too high");
        ensure!(avg <= max && avg >= min, "avg chunk size is out of range");
        Ok(ChunkSize { min, avg, max })
    }
}

pub struct Chunk {
    offset: u64,
    id: format::ChunkId,
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Chunk {{ off: x{:x}, id: {:x}",
            self.offset,
            self.id.as_hex()
        )
    }
}

pub fn read_index<R: Read, F>(mut from: R, mut into: F) -> Result<()>
where
    F: FnMut(Chunk) -> Result<()>,
{
    {
        let header_size = leu64(&mut from)?;
        ensure!(
            48 == header_size,
            "file size doesn't look like a supported index: {}",
            header_size
        );
    }

    ensure!(
        format::INDEX == leu64(&mut from)?,
        "file magic number doesn't look like an index"
    );

    let feature_flags = leu64(&mut from)?;
    let chunk_size = ChunkSize::new(leu64(&mut from)?, leu64(&mut from)?, leu64(&mut from)?)?;

    ensure!(
        std::u64::MAX == leu64(&mut from)?,
        "table size should be u64::MAX"
    );

    ensure!(format::TABLE == leu64(&mut from)?, "table magic missing");

    loop {
        let offset = leu64(&mut from)?;
        let mut id = format::ChunkId::default();
        from.read_exact(&mut id)?;

        // TODO: other conditions to validate we're actually at the end
        if 0 == offset && [0u8; 8] == id[0..8] {
            let mut single_byte = [0u8; 1];
            match from.read_exact(&mut single_byte) {
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                _ => bail!("end of index marker, but not at end of file"),
            }
        }


        into(Chunk { offset, id })?;
    }
    Ok(())
}

fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    from.read_u64::<LittleEndian>()
}
