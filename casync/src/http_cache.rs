use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::io::Read;
use std::io::Write;

use failure::bail;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use reqwest::Client;
use reqwest::IntoUrl;

pub struct HttpCache<'c> {
    client: &'c Client,
    local_store: PathBuf,
}

impl<'c> HttpCache<'c> {
    pub fn new<P: AsRef<Path>>(client: &'c Client, local_store: P) -> Result<Self, Error> {
        Ok(HttpCache {
            client,
            local_store: local_store.as_ref().to_path_buf(),
        })
    }

    pub fn load<U: IntoUrl>(&self, castr: U, cacnk: &str) -> Result<Vec<u8>, Error> {
        let mut chunk_path = self.local_store.to_path_buf();
        chunk_path.push(cacnk);

        match fs::read(&chunk_path) {
            Ok(v) => return Ok(v),
            Err(ref e) if io::ErrorKind::NotFound == e.kind() => (),
            Err(e) => Err(e)?,
        }

        let castr = castr.into_url()?;
        let cacnk = castr.join(cacnk)?;

        fs::create_dir_all(chunk_path.parent().unwrap())?;

        let mut resp = self.client.get(cacnk.clone()).send()?;

        // TODO: give up again if the file already exists

        if !resp.status().is_success() {
            bail!("couldn't download chunk: {}\nurl: {}", resp.status(), cacnk);
        }

        let mut buf = Vec::with_capacity(resp.content_length().unwrap_or(8 * 1024) as usize);
        resp.read_to_end(&mut buf)?;

        let mut temp =
            tempfile_fast::PersistableTempFile::new_in(&self.local_store).with_context(|_| {
                format_err!("creating temporary directory inside {:?}", self.local_store)
            })?;
        temp.write_all(&buf)?;

        // TODO: ignore already-exists errors
        temp.persist_noclobber(&chunk_path)
            .map_err(|e| e.error)
            .with_context(|_| format_err!("storing downloaded chunk into: {:?}", chunk_path))?;

        Ok(buf)
    }

    pub fn local_store(&self) -> &Path {
        &self.local_store
    }
}
