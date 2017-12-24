use std;
use std::io;
use std::io::Read;
use std::fmt;

use errors::*;
use format::StreamMagic;

use byteorder::{LittleEndian, ReadBytesExt};

const HEADER_TAG_LEN: u64 = 16;

#[derive(Default)]
pub struct Entry {
    pub mode: u64,
    pub uid: u64,
    pub gid: u64,
    pub mtime: u64,
    pub user_name: Option<Vec<u8>>,
    pub group_name: Option<Vec<u8>>,
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Entry {{ 0o{:o}{} u:{} g:{} user: {:?} group: {:?} }}",
            self.mode & 0o7777,
            if self.is_dir() {
                "d"
            } else if self.is_reg() {
                "r"
            } else {
                " XXX"
            },
            self.uid,
            self.gid,
            self.user_name,
            self.group_name
        )
    }
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        0o040000 == (self.mode & 0o170000)
    }

    pub fn is_reg(&self) -> bool {
        0o100000 == (self.mode & 0o170000)
    }
}

pub fn read_stream<R: Read, F>(mut from: R, mut into: F) -> Result<()>
where
    F: FnMut(&[Vec<u8>], Entry, Option<io::Take<&mut R>>) -> Result<()>,
{
    let mut current: Option<Entry> = None;
    // State machine here is maintained using the depth of path;
    // when we see a goodbye and that leaves the path array empty,
    // we're at the end of the archive

    let mut path = vec![];
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;
        //println!("header: {:?}", header_format);
        match header_format {
            StreamMagic::Entry => {
                ensure!(
                    (8 * 6) + HEADER_TAG_LEN == header_size,
                    "incorrect ENTRY length; not supported by us: 48 != {}",
                    header_size
                );

                ensure!(current.is_none(), "entry found without data");
                current = Some(load_entry(&mut from)?);
            }
            StreamMagic::User => {
                current.as_mut().ok_or("user without entry")?.user_name =
                    Some(read_string_record(header_size, &mut from)?);
            }
            StreamMagic::Group => {
                current.as_mut().ok_or("group without entry")?.group_name =
                    Some(read_string_record(header_size, &mut from)?);
            }
            StreamMagic::Name => {
                let mut new_name = read_data_record(header_size, &mut from)?;

                ensure!(
                    new_name
                        .pop()
                        .map(|last_char| 0 == last_char)
                        .unwrap_or(false),
                    "filename must be non-empty and null-terminated"
                );

                if let Some(current) = current {
                    // if we're currently in an Entry, then a new filename indicates a new,
                    // nested archive, which continues until the Goodbye pops it off the end
                    into(&path, current, None)?;
                    path.push(new_name);
                } else {
                    // if we're not in an entry, we're just updating the current filename
                    let last_element = path.len() - 1;
                    path[last_element] = new_name;
                }

                current = None;
            }
            StreamMagic::Data => {
                ensure!(
                    header_size >= HEADER_TAG_LEN,
                    "data <0 bytes long: {}",
                    header_size
                );
                into(
                    &path,
                    current.ok_or("payload without entry")?,
                    Some((&mut from).take(header_size - HEADER_TAG_LEN)),
                )?;
                current = None;
            }
            StreamMagic::Bye => {
                // TODO: all kinds of tailing records
                read_data_record(header_size, &mut from)?;
                path.pop();
                if path.is_empty() {
                    return Ok(());
                }
            }
        }
    }
}

fn load_entry<R: Read>(mut from: R) -> Result<Entry> {
    let mut entry = Entry::default();

    leu64(&mut from)?; // feature_flags

    entry.mode = leu64(&mut from)?;

    leu64(&mut from)?; // flags

    entry.uid = leu64(&mut from)?;
    entry.gid = leu64(&mut from)?;
    entry.mtime = leu64(&mut from)?;

    Ok(entry)
}

pub fn dump_packets<R: Read>(mut from: R) -> Result<()> {
    let mut in_entry = false;
    let mut depth = 0usize;
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;

        let payload_len = header_size - 16;
        let mut payload = vec![0; usize_from(payload_len)];
        from.read_exact(&mut payload)?;
        print!(
            "{} * {:5} | {:3} | ",
            String::from_utf8(vec![b' '; depth * 2]).unwrap(),
            format!("{:?}", header_format),
            payload_len
        );

        match header_format {
            StreamMagic::Entry => {
                let entry = load_entry(io::Cursor::new(&payload))?;
                println!("dir: {}", entry.is_dir());
            }
            StreamMagic::Data => {
                println!();

                depth -= 1;
            }
            StreamMagic::Name => {
                println!("{}", String::from_utf8_lossy(&payload[..payload.len() - 1]));
                depth += 1;
            }
            StreamMagic::Bye => {
                println!();
                depth -= 1;

                if 0 == depth {
                    return Ok(());
                }
            }
            _ => {
                println!("{}", String::from_utf8_lossy(&payload[..payload.len() - 1]));
            }
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

pub fn utf8_path(from: &[Vec<u8>]) -> std::result::Result<String, ::std::string::FromUtf8Error> {
    let mut ret = String::new();
    for component in from {
        ret.push_str(String::from_utf8(component.clone())?.as_str());
        ret.push_str("/");
    }

    if !ret.is_empty() {
        let waste = ret.len() - 1;
        ret.truncate(waste);
    }

    Ok(ret)
}

fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    from.read_u64::<LittleEndian>()
}

fn usize_from(val: u64) -> usize {
    assert!(val <= std::usize::MAX as u64);
    val as usize
}
