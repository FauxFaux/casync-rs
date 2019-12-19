use std::fs;
use std::io;
use std::io::Write;

use casync_format::Chunk;
use failure::bail;
use failure::ensure;
use failure::err_msg;
use failure::format_err;
use failure::Error;
use failure::ResultExt;

pub fn fast_export<W: Write>(mut into: W, castr: &str, caidx: &str) -> Result<(), Error> {
    let file = fs::File::open(caidx).with_context(|_| err_msg("opening index file"))?;

    let mut v: Vec<Chunk> = vec![];
    let (_sizes, v) =
        casync_format::read_index(file).with_context(|_| err_msg("reading index file"))?;

    let mut it = v.into_iter();

    let reader = casync_format::ChunkReader::new(|| {
        Ok(match it.next() {
            Some(chunk) => Some(chunk.open_from(castr)?),
            None => None,
        })
    })
    .with_context(|_| err_msg("initialising reader"))?;

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    let mut stream = casync_format::Stream::new(reader);
    while let Some(path_content) = stream
        .next()
        .with_context(|_| format_err!("reading stream of index {}", caidx))?
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
