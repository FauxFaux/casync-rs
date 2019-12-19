use std::collections::VecDeque;
use std::io;
use std::io::Read;

use failure::bail;
use failure::err_msg;
use failure::Error;
use failure::ResultExt;

pub struct FlatReader<I> {
    inner: I,
    buf: VecDeque<u8>,
}

impl<I> FlatReader<I>
where
    I: Iterator<Item = Result<Vec<u8>, io::Error>>,
{
    pub fn new(inner: I) -> FlatReader<I> {
        FlatReader {
            inner,
            buf: VecDeque::with_capacity(1024 * 16),
        }
    }
}

impl<I> Read for FlatReader<I>
where
    I: Iterator<Item = Result<Vec<u8>, io::Error>>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        // if we haven't got any data, consume from the iterator
        // if the iterator is also empty, we can't read
        while self.buf.is_empty() {
            match self.inner.next() {
                Some(v) => self.buf.extend(v?),
                None => return Ok(0),
            }
        }

        // get at least some of the VecDeque as a slice
        let (start, end) = self.buf.as_slices();
        let from = if !start.is_empty() { start } else { end };

        // decide how much we're going to read
        let reading = buf.len().min(from.len());

        // read it, and discard it from our buffer
        buf[..reading].copy_from_slice(&from[..reading]);
        self.buf.drain(..reading);

        Ok(reading)
    }
}

pub struct ChunkReader<R, F> {
    inner: R,
    next: F,
}

impl<R: Read, F> ChunkReader<R, F>
where
    F: FnMut() -> io::Result<Option<R>>,
{
    pub fn new(mut from: F) -> Result<Self, Error> {
        Ok(ChunkReader {
            inner: match from().with_context(|_| err_msg("trying to fetch initial chunk"))? {
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

    use crate::chunks::ChunkReader;

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
