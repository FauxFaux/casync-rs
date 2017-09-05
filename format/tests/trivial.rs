extern crate casync_format;

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

    let reader = casync_format::ChunkReader::new(|| {
        Ok(it.next().map(
            |chunk| chunk.open_from("tests/data/nums.castr")?,
        ))
    }).unwrap();

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    casync_format::read_stream(reader, |path, entry, data| {
        println!("{}, {:?}", path.len(), entry);
        let mut buf = vec![];
        if data.is_some() {
            data.unwrap().read_to_end(&mut buf)?;
        }
        Ok(())
    }).unwrap();
}
