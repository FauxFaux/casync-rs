use casync_format;
use tempfile_fast;

use std::mem;
use std::io;
use std::path;

use casync_format::Chunk;
use reqwest::Client;
use reqwest::header;

use errors::*;

pub struct Fetcher {
    client: Client,
    mirror_root: String,
    local_store: path::PathBuf,
    remote_store: String,
}

impl Fetcher {
    pub fn parse_whole_index(&self, rel_path: String) -> Result<Vec<Chunk>> {
        let uri = format!("{}{}", self.mirror_root, rel_path);

        let resp = self.client.get(&uri)?.send()?;

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

    pub fn fetch_all_chunks<I>(&self, chunks: I) -> Result<()>
    where
        I: Iterator<Item = Chunk>,
    {
        for chunk in chunks {
            let mut chunk_path = self.local_store.clone();
            chunk_path.push(chunk.format_id());
            if chunk_path.is_file() {
                // we already have it
                continue;
            }

            let uri = format!(
                "{}{}{}",
                self.mirror_root,
                self.remote_store,
                chunk.format_id()
            );
            let mut resp = self.client.get(&uri)?.send()?;
            if !resp.status().is_success() {
                bail!("couldn't download chunk: {}", resp.status());
            }

            let mut temp = tempfile_fast::persistable_tempfile_in(&self.local_store)?;
            let written = io::copy(&mut resp, temp.as_mut())?;

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

            temp.persist_noclobber(chunk_path)?;
        }

        Ok(())
    }
}
