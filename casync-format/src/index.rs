use std;
use std::fmt;
use std::io;
use std::io::Read;

use anyhow::bail;
use anyhow::ensure;
use anyhow::Error;

use crate::format::ChunkId;
use crate::format::IndexMagic;

pub struct ChunkSize {
    pub min: u64,
    pub avg: u64,
    pub max: u64,
}

impl ChunkSize {
    fn new(min: u64, avg: u64, max: u64) -> Result<ChunkSize, Error> {
        ensure!(min >= 1, "minimum chunk size is too low");
        ensure!(max <= 128 * 1024 * 1024, "maximum chunk size is too high");
        ensure!(avg <= max && avg >= min, "avg chunk size is out of range");
        Ok(ChunkSize { min, avg, max })
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Chunk {
    pub offset: u64,
    pub id: ChunkId,
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Chunk {{ off: x{:x}, id: {} }}",
            self.offset,
            self.format_id()
        )
    }
}

pub fn format_chunk_id(id: &ChunkId) -> String {
    let mut ret = format!("{:02x}{:02x}/", id[0], id[1]);
    for byte in id {
        ret.push_str(format!("{:02x}", byte).as_str());
    }
    ret.push_str(".cacnk");
    ret
}

impl Chunk {
    pub fn format_id(&self) -> String {
        format_chunk_id(&self.id)
    }

    pub fn check(&self, data: &[u8]) -> io::Result<()> {
        let actual = digest(data);

        if actual != self.id {
            return Err(io::Error::new(io::ErrorKind::Other, "checksum mismatch"));
        }

        Ok(())
    }
}

pub fn read_index<R: Read>(mut from: R) -> Result<(ChunkSize, Vec<Chunk>), Error> {
    {
        let header_size = leu64(&mut from)?;
        ensure!(
            48 == header_size,
            "file size doesn't look like a supported index: {}",
            header_size
        );
    }

    ensure!(
        IndexMagic::Index == IndexMagic::from(leu64(&mut from)?)?,
        "file magic number doesn't look like an index"
    );

    leu64(&mut from)?; // feature_flags
    let chunk_size = {
        let min = leu64(&mut from)?;
        let avg = leu64(&mut from)?;
        let max = leu64(&mut from)?;
        ChunkSize::new(min, avg, max)?
    };

    ensure!(
        std::u64::MAX == leu64(&mut from)?,
        "table size should be u64::MAX"
    );

    ensure!(
        IndexMagic::Table == IndexMagic::from(leu64(&mut from)?)?,
        "table magic missing"
    );

    let mut chunks = Vec::with_capacity(32);

    loop {
        let offset = leu64(&mut from)?;
        let mut id = ChunkId::default();
        from.read_exact(&mut id)?;

        // TODO: other conditions to validate we're actually at the end
        if 0 == offset && [0u8; 8] == id[0..8] {
            if at_eof(from)? {
                break;
            }
            bail!("end of index marker, but not at end of file")
        }

        chunks.push(Chunk { offset, id });
    }

    Ok((chunk_size, chunks))
}

fn at_eof<R: Read>(mut from: R) -> io::Result<bool> {
    let mut single_byte = [0u8; 1];
    match from.read_exact(&mut single_byte) {
        Ok(_) => Ok(false),
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(true),
        Err(e) => Err(e),
    }
}

fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    from.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn digest(data: &[u8]) -> ChunkId {
    use sha2::Digest;
    let digest = sha2::Sha512Trunc256::digest(data);
    let mut id = ChunkId::default();
    id.copy_from_slice(&digest[..]);
    id
}
