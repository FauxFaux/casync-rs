use std::io;
use std::io::Read;

use errors::*;

pub struct ChunkReader<R, F> {
    inner: R,
    next: F,
}

impl<R: Read, F> ChunkReader<R, F>
where
    F: FnMut() -> io::Result<Option<R>>,
{
    pub fn new(mut from: F) -> Result<Self> {
        Ok(ChunkReader {
            inner: match from().chain_err(|| "trying to fetch initial chunk")? {
                Some(reader) => reader,
                None => bail!("there must be at least one chunk"),
            },
            next: from,
        })
    }
}

impl<R: Read, F> Read for ChunkReader<R, F>
where
    F: FnMut() -> io::Result<Option<R>>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(r) if 0 != r => Ok(r),
            Ok(0) => {
                self.inner = match (self.next)()? {
                    Some(reader) => reader,
                    None => return Ok(0),
                };
                self.inner.read(buf)
            }
            Err(e) => Err(e),
            Ok(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::Read;
    use chunks::ChunkReader;

    #[test]
    fn cursors() {
        let inputs = vec![io::Cursor::new([0, 1]), io::Cursor::new([2, 3])];
        let mut it = inputs.into_iter();
        let mut r = ChunkReader::new(|| Ok(it.next())).unwrap();
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf).unwrap();
        assert_eq!([0, 1, 2, 3], buf);
    }
}
