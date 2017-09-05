use std::io;
use std::io::Read;

use errors::*;
use format;

use byteorder::{ReadBytesExt, LittleEndian};

const HEADER_TAG_LEN: u64 = 16;

#[derive(Debug, Default)]
pub struct Entry {
    pub mode: u64,
    pub uid: u64,
    pub gid: u64,
    pub mtime: u64,
    pub user_name: Option<Vec<u8>>,
    pub group_name: Option<Vec<u8>>,
}

pub fn read_stream<R: Read, F>(mut from: R, mut into: F) -> Result<()>
where
    F: FnMut(&[Vec<u8>], Entry, Option<io::Take<&mut R>>) -> Result<()> {
    let mut current: Option<Entry> = None;
    // State machine here is maintained using the depth of path;
    // when we see a goodbye and that leaves the path array empty,
    // we're at the end of the archive

    let mut path = vec![];
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = leu64(&mut from)?;
//        println!("{:x}", header_format);
        match header_format {
            format::ENTRY => {
                ensure!(
                    (8 * 6) + HEADER_TAG_LEN == header_size,
                    "incorrect ENTRY length; not supported by us: 48 != {}",
                    header_size
                );

                ensure!(current.is_none(), "entry found without data");
                let mut entry = Entry::default();

                leu64(&mut from)?; // feature_flags

                entry.mode = leu64(&mut from)?;

                leu64(&mut from)?; // flags

                entry.uid = leu64(&mut from)?;
                entry.gid = leu64(&mut from)?;
                entry.mtime = leu64(&mut from)?;

                current = Some(entry);
            }
            format::USER => {
                current
                    .as_mut()
                    .ok_or("user without entry")?
                    .user_name = Some(read_string_record(header_size, &mut from)?);
            }
            format::GROUP => {
                current
                    .as_mut()
                    .ok_or("group without entry")?
                    .group_name = Some(read_string_record(header_size, &mut from)?);
            }
            format::FILENAME => {
                into(&path, current.ok_or("filename without entry")?, None)?;

                path.push(read_data_record(header_size, &mut from)?);
                current = None;
            }
            format::PAYLOAD => {
                ensure!(header_size >= HEADER_TAG_LEN, "data <0 bytes long: {}", header_size);
                into(&path, current.ok_or("payload without entry")?, Some((&mut from).take(header_size - HEADER_TAG_LEN)))?;
                current = None;
            }
            format::GOODBYE => {
                // TODO: all kinds of tailing records
                read_data_record(header_size, &mut from)?;
                path.pop();
                if path.is_empty() {
                    return Ok(());
                }
            }
            _ => bail!("unrecognised header format: 0x{:016x}", header_format),
        }
    }
}

fn read_string_record<R: Read>(header_size: u64, from: R) -> Result<Vec<u8>> {
    ensure!(
        header_size < 256 + HEADER_TAG_LEN,
        "refusing to support names over ~255 characters, was: {}",
        header_size
    );
    read_data_record(header_size, from)
}

fn read_data_record<R: Read>(header_size: u64, mut from: R) -> Result<Vec<u8>> {
    ensure!(
        header_size >= HEADER_TAG_LEN,
        "header missing / size wrong: {}",
        header_size
    );

    let mut buf = vec![0u8; (header_size - HEADER_TAG_LEN) as usize];
    from.read_exact(&mut buf)?;
    Ok(buf)
}


fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    from.read_u64::<LittleEndian>()
}
