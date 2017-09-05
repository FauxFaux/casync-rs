use std;
use std::fmt;
use std::io;
use std::io::Read;

use errors::*;
use format;

use byteorder::{ReadBytesExt, LittleEndian};

const HEADER_TAG_LEN: u64 = 16;

pub fn read_stream<R: Read>(mut from: R) -> Result<()> {
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = leu64(&mut from)?;

        match header_format {
            format::ENTRY => {
                ensure!(
                    (8 * 6) + HEADER_TAG_LEN == header_size,
                    "incorrect ENTRY length; not supported by us: 48 != {}",
                    header_size
                );
                leu64(&mut from)?; // feature_flags
                leu64(&mut from)?; // mode
                leu64(&mut from)?; // flags
                leu64(&mut from)?; // uid
                leu64(&mut from)?; // gid
                leu64(&mut from)?; // mtime
            }
            format::USER => {
                read_string_record(header_size, &mut from)?; // name
            }
            format::GROUP => {
                read_string_record(header_size, &mut from)?; // name
            }
            format::FILENAME => {
                println!(
                    "filename: {}",
                    String::from_utf8(read_data_record(header_size, &mut from)?)?
                ); // name
            }
            format::PAYLOAD => {
                println!("data: {}", read_data_record(header_size, &mut from)?.len()); // data (huge?)
            }
            format::GOODBYE => {
                // TODO: all kinds of tailing records
                return Ok(());
            }
            _ => bail!("unrecognised header format: 0x{:016x}", header_format),
        }
    }
}

fn read_string_record<R: Read>(header_size: u64, from: R) -> Result<Vec<u8>> {
    ensure!(
        header_size < 256 + HEADER_TAG_LEN && header_size >= HEADER_TAG_LEN,
        "refusing to support names over ~255 characters, was: {}",
        header_size
    );
    read_data_record(header_size, from)
}

fn read_data_record<R: Read>(header_size: u64, mut from: R) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; (header_size - HEADER_TAG_LEN) as usize];
    from.read_exact(&mut buf)?;
    Ok(buf)
}


fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    from.read_u64::<LittleEndian>()
}
