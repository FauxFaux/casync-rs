extern crate casync_format;
extern crate zstd;

use std::io;
use std::fs;

use std::io::Read;

#[test]
fn load_index() {
    let file = io::Cursor::new(&include_bytes!("data/trivial.caidx")[..]);
    let mut v = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).unwrap();

    for chunk in v {
        println!("{:?}", chunk)
    }
}

/// rm -rf nums; mkdir nums && seq 10000 > nums/data && casync make --store=nums.castr nums.caidx nums
#[test]
fn load_nums() {
    let file = fs::File::open("tests/data/nums.caidx").unwrap();
    let mut v = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).unwrap();

    let mut it = v.into_iter();

    let local_store_root = "tests/data/nums.castr";

    let mut reader = casync_format::ChunkReader::new(|| {
        Ok(match it.next() {
            None => None,
            Some(chunk) => {
                Some(zstd::Decoder::new(fs::File::open(format!(
                    "{}/{}.cacnk",
                    local_store_root,
                    chunk.format_id()
                ))?)?)
            }
        })
    }).unwrap();

    let mut buf = vec![];
    assert_eq!(49207, reader.read_to_end(&mut buf).unwrap());
}
