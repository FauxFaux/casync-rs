extern crate casync_format;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate tempfile_fast;

use std::fs;
use std::io;
use std::path;

use std::io::Write;

use casync_format::Chunk;
use futures::{Future, Stream};
use hyper::Client;
use hyper::client::Connect;

mod errors;

use errors::*;

// TODO: take a reactor and add some downloads to it

// TODO: take an index and add all the downloads to it

// TODO: take a load of indexes and download them all

pub struct Fetcher<C: Connect> {
    client: Client<C>,
    mirror_root: String,
    local_store: path::PathBuf,
    remote_store: String,
}

impl<C: Connect> Fetcher<C> {
    pub fn parse_whole_index(
        &self,
        rel_path: String,
    ) -> Result<Box<Future<Item = Vec<Chunk>, Error = hyper::Error>>> {
        let uri = format!("{}{}", self.mirror_root, rel_path).parse()?;

        Ok(Box::new(self.client.get(uri).and_then(|resp| {
            resp.body().concat2().and_then(|body| {
                let mut chunks = Vec::new();
                casync_format::read_index(io::Cursor::new(&body), |found| {
                    chunks.push(found);
                    Ok(())
                }).map_err(|casync_error| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "sorry no error handling for you, the type system is too hard: {}",
                            casync_error
                        ),
                    )
                })?;
                Ok(chunks)
            })
        })))
    }

    pub fn fetch_all_chunks<I>(
        &self,
        chunks: I,
    ) -> Result<Box<Future<Item = (), Error = hyper::Error>>>
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

            let uri = format!("{}{}", self.mirror_root, chunk.format_id()).parse()?;
            self.client.get(uri).and_then(|resp| {
                let mut temp = tempfile_fast::persistable_tempfile_in(&self.local_store).expect("error handling");

                resp.body().for_each(|chunk|
                    temp.write_all(&chunk).map(|_| ()).map_err(From::from)
                ).then(|status| {
                    temp.persist_noclobber(chunk_path).map_err(From::from)
                })
            });
        }
        unreachable!()
    }
}
