use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;

use anyhow::bail;
use anyhow::ensure;
use anyhow::format_err;
use anyhow::Context;
use anyhow::Error;

use casync_format::chunks::from_paths;
use casync_format::Stream;

pub fn fast_export<W: Write>(mut into: W, castr: &str, caidx: &str) -> Result<(), Error> {
    let mut stream = Stream::new(from_paths(caidx, castr, move |path: &str| fs::read(path))?);

    while let Some(path_content) = stream
        .next()
        .with_context(|| format_err!("reading stream of index {}", caidx))?
    {
        let (path, content) = path_content;
        let last = path.end().clone();
        let names: Vec<Box<[u8]>> = path.into_iter().map(|item| item.name).collect();

        let last_entry = match last.entry {
            Some(x) => x,
            None => bail!("no entry for item"),
        };

        match content {
            casync_format::Content::File(mut data) => {
                ensure!(last_entry.is_reg(), "TODO: data for non-regular file");

                let executable = 0o100 == (last_entry.mode & 0o100);

                writeln!(
                    into,
                    "M {} inline {}",
                    if executable { "100755" } else { "100644" },
                    casync_format::utf8_path(names)?
                )?;
                writeln!(into, "data {}", data.limit())?;
                io::copy(&mut data, &mut io::stdout())?;
            }
            casync_format::Content::Directory => {
                ensure!(last_entry.is_dir(), "directory end for non-directory");
            }
        }
    }
    Ok(())
}

pub fn mtree<W: Write>(mut into: W, castr: &str, caidx: &str) -> Result<(), Error> {
    let mut stream = Stream::new(from_paths(caidx, castr, move |path: &str| fs::read(path))?);

    while let Some(path_content) = stream
        .next()
        .with_context(|| format_err!("reading stream of index {}", caidx))?
    {
        let (path, content) = path_content;
        let last = path.end().clone();
        let names: Vec<Box<[u8]>> = path.into_iter().map(|item| item.name).collect();
        writeln!(into, "{}, {:?}", casync_format::utf8_path(names)?, last)?;

        match content {
            casync_format::Content::File(mut data) => {
                let mut buf = Vec::new();
                data.read_to_end(&mut buf)?;
            }
            casync_format::Content::Directory => {}
        }
    }
    Ok(())
}
