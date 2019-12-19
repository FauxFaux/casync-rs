use std::fs;
use std::io;
use std::path;

use casync_format::format_chunk_id;
use casync_format::Chunk;
use casync_format::ChunkId;
use failure::bail;
use failure::ensure;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use reqwest::Client;

pub struct Fetcher<'c> {
    client: &'c Client,
    mirror_root: String,
    local_store: path::PathBuf,
    remote_store: String,
}

impl<'c> Fetcher<'c> {
    pub fn new<P: AsRef<path::Path>>(
        client: &'c Client,
        mirror_root: &str,
        local_store: P,
        remote_store: &str,
    ) -> Result<Self, Error> {
        Ok(Fetcher {
            client,
            mirror_root: mirror_root.to_string(),
            local_store: local_store.as_ref().to_path_buf(),
            remote_store: remote_store.to_string(),
        })
    }

    pub fn parse_whole_index(&self, rel_path: String) -> Result<Vec<Chunk>, Error> {
        let uri = format!("{}{}", self.mirror_root, rel_path);

        let resp = self.client.get(&uri).send()?;

        if !resp.status().is_success() {
            bail!("request failed: {}", resp.status());
        }
        let (_sizes, chunks) = casync_format::read_index(resp)?;

        Ok(chunks)
    }

    pub fn fetch_all_chunks<'a, I>(&self, chunks: I) -> Result<(), Error>
    where
        I: Iterator<Item = &'a ChunkId>,
    {
        for chunk in chunks {
            let mut chunk_path = self.local_store.clone();
            chunk_path.push(format_chunk_id(&chunk));
            if chunk_path.is_file() {
                // we already have it
                continue;
            }

            let uri = format!(
                "{}{}/{}",
                self.mirror_root,
                self.remote_store,
                format_chunk_id(&chunk),
            );
            let mut resp = self.client.get(&uri).send()?;

            // TODO: give up again if the file already exists

            if !resp.status().is_success() {
                bail!("couldn't download chunk: {}\nurl: {}", resp.status(), uri);
            }

            let mut temp = tempfile_fast::PersistableTempFile::new_in(&self.local_store)
                .with_context(|_| {
                    format_err!("creating temporary directory inside {:?}", self.local_store)
                })?;
            let written = io::copy(&mut resp, &mut temp)?;

            if let Some(expected) = resp.content_length() {
                ensure!(
                    written == expected,
                    "data wasn't the right length, actual: {}, expected: {}",
                    written,
                    expected
                );
            }

            fs::create_dir_all(chunk_path.parent().unwrap())?;

            // TODO: ignore already-exists errors
            temp.persist_noclobber(&chunk_path)
                .map_err(|e| e.error)
                .with_context(|_| format_err!("storing downloaded chunk into: {:?}", chunk_path))?;
        }

        Ok(())
    }

    pub fn local_store(&self) -> path::PathBuf {
        self.local_store.clone()
    }

    #[cfg(never)]
    pub fn read_cache<'r, I, F>(&self, mut chunks: I, into: F) -> Result<(), Error>
    where
        I: Iterator<Item = Chunk>,
        F: FnMut(
            &'r [Vec<u8>],
            casync_format::Entry,
            Option<Box<dyn io::Read>>,
        ) -> casync_format::Result<()>,
    {
        let reader = casync_format::ChunkReader::new(|| {
            Ok(match chunks.next() {
                Some(chunk) => Some(chunk.open_from(self.local_store)?),
                None => None,
            })
        })
        .with_context(|_| err_msg("initialising reader"))?;

        casync_format::read_stream(reader, |v, e, r| {
            into(v, e, r.map(|t| Box::new(t) as Box<dyn io::Read>))?;
            Ok(())
        })
        .with_context(|_| err_msg("reading stream"))
    }
}
