use casync_format;
use tempfile_fast;

use std::fs;
use std::mem;
use std::io;
use std::path;

use casync_format::Chunk;
use casync_format::format_chunk_id;
use casync_format::ChunkId;
use reqwest::Client;
use reqwest::header;

use errors::*;

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
    ) -> Result<Self> {
        Ok(Fetcher {
            client,
            mirror_root: mirror_root.to_string(),
            local_store: local_store.as_ref().to_path_buf(),
            remote_store: remote_store.to_string(),
        })
    }

    pub fn parse_whole_index(&self, rel_path: String) -> Result<Vec<Chunk>> {
        let uri = format!("{}{}", self.mirror_root, rel_path);

        let resp = self.client.get(&uri).send()?;

        if !resp.status().is_success() {
            bail!("request failed: {}", resp.status());
        }
        let estimated_length = match resp.headers().get::<header::ContentLength>() {
            Some(&header::ContentLength(len)) => len,
            _ => 1337,
        };

        let estimated_length = estimated_length as usize / mem::size_of::<Chunk>();
        let mut chunks = Vec::with_capacity(estimated_length);

        casync_format::read_index(resp, |chunk| {
            chunks.push(chunk);
            Ok(())
        })?;

        Ok(chunks)
    }

    pub fn fetch_all_chunks<'a, I>(&self, chunks: I) -> Result<()>
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
                .chain_err(|| {
                    format!("creating temporary directory inside {:?}", self.local_store)
                })?;
            let written = io::copy(&mut resp, &mut temp)?;

            if let Some(&header::ContentLength(expected)) =
                resp.headers().get::<header::ContentLength>()
            {
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
                .chain_err(|| format!("storing downloaded chunk into: {:?}", chunk_path))?;
        }

        Ok(())
    }

    pub fn local_store(&self) -> path::PathBuf {
        self.local_store.clone()
    }

    #[cfg(never)]
    pub fn read_cache<'r, I, F>(&self, mut chunks: I, into: F) -> Result<()>
    where
        I: Iterator<Item = Chunk>,
        F: FnMut(&'r [Vec<u8>], casync_format::Entry, Option<Box<io::Read>>)
            -> casync_format::Result<()>,
    {
        let reader = casync_format::ChunkReader::new(|| {
            Ok(match chunks.next() {
                Some(chunk) => Some(chunk.open_from(self.local_store)?),
                None => None,
            })
        }).chain_err(|| "initialising reader")?;

        casync_format::read_stream(reader, |v, e, r| {
            into(v, e, r.map(|t| Box::new(t) as Box<io::Read>))?;
            Ok(())
        }).chain_err(|| "reading stream")
    }
}
