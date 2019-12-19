use std::collections::VecDeque;
use std::io;
use std::io::Read;

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
