use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

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

    pub async fn load<U: IntoUrl>(&self, castr: U, cacnk: &str) -> Result<Vec<u8>, Error> {
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

        let resp = self.client.get(cacnk.clone()).send().await?;

        // TODO: give up again if the file already exists

        if !resp.status().is_success() {
            bail!("couldn't download chunk: {}\nurl: {}", resp.status(), cacnk);
        }

        // TODO: chunks() to file, or read.await
        let buf = resp.bytes().await?.to_vec();

        let mut temp =
            tempfile_fast::PersistableTempFile::new_in(&self.local_store).with_context(|_| {
                format_err!("creating temporary directory inside {:?}", self.local_store)
            })?;
        temp.write_all(&buf)?;

        match temp.persist_noclobber(&chunk_path).map_err(|e| e.error) {
            Ok(_) => (),
            Err(ref e) if io::ErrorKind::AlreadyExists == e.kind() => (),
            Err(e) => Err(e)
                .with_context(|_| format_err!("storing downloaded chunk into: {:?}", chunk_path))?,
        }

        Ok(buf)
    }

    pub fn local_store(&self) -> &Path {
        &self.local_store
    }
}
