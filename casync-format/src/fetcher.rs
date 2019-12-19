use std::io;

pub trait Fetcher {
    fn fetch(&mut self, path: &str) -> Result<Vec<u8>, io::Error>;
}

impl<T> Fetcher for T
where
    T: FnMut(&str) -> Result<Vec<u8>, io::Error>,
{
    fn fetch(&mut self, path: &str) -> Result<Vec<u8>, io::Error> {
        self(path)
    }
}
