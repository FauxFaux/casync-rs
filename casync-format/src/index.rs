use std;
use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path;

use byteorder::ReadBytesExt;
use byteorder::LE;
use failure::bail;
use failure::ensure;
use failure::Error;

use crate::format::ChunkId;
use crate::format::IndexMagic;

struct ChunkSize {
    min: u64,
    avg: u64,
    max: u64,
}

impl ChunkSize {
    fn new(min: u64, avg: u64, max: u64) -> Result<ChunkSize, Error> {
        ensure!(min >= 1, "minimum chunk size is too low");
        ensure!(max <= 128 * 1024 * 1024, "maximum chunk size is too high");
        ensure!(avg <= max && avg >= min, "avg chunk size is out of range");
        Ok(ChunkSize { min, avg, max })
    }
}

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

    pub fn open_from<P: AsRef<path::Path>>(
        &self,
        castr_path: P,
    ) -> io::Result<zstd::Decoder<io::BufReader<fs::File>>> {
        let mut buf = castr_path.as_ref().to_path_buf();
        buf.push(self.format_id());
        zstd::Decoder::new(fs::File::open(buf)?)
    }
}

pub fn read_index<R: Read, F>(mut from: R, mut into: F) -> Result<(), Error>
where
    F: FnMut(Chunk) -> Result<(), Error>,
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
        IndexMagic::Index == IndexMagic::from(leu64(&mut from)?)?,
        "file magic number doesn't look like an index"
    );

    leu64(&mut from)?; // feature_flags
    ChunkSize::new(leu64(&mut from)?, leu64(&mut from)?, leu64(&mut from)?)?; // chunk_size

    ensure!(
        std::u64::MAX == leu64(&mut from)?,
        "table size should be u64::MAX"
    );

    ensure!(
        IndexMagic::Table == IndexMagic::from(leu64(&mut from)?)?,
        "table magic missing"
    );

    loop {
        let offset = leu64(&mut from)?;
        let mut id = ChunkId::default();
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
    from.read_u64::<LE>()
}
