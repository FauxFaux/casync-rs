extern crate casync_format;
extern crate clap;
#[macro_use]
extern crate error_chain;

mod errors;

use std::fs;
use std::io::Read;

use casync_format::Chunk;
use clap::{Arg, App, AppSettings, SubCommand};

use errors::*;

fn run() -> Result<()> {
    let matches = App::new("casync-rs")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("mtree")
                .about("dump data about some archives")
                .arg(
                    Arg::with_name("CAIDX")
                        .help("the index file(s) to inspect")
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("store")
                        .help("the castore which the indexes reference")
                        .long("store")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("mtree") {
        let castr = matches.value_of("store").unwrap();
        for idx in matches.values_of("CAIDX").unwrap() {
            mtree(castr, idx)?;
        }
    } else {
        unreachable!();
    }

    Ok(())
}

fn mtree(castr: &str, caidx: &str) -> Result<()> {
    let file = fs::File::open(caidx).chain_err(|| "opening index file")?;

    let mut v: Vec<Chunk> = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).chain_err(|| "reading index file")?;

    let mut it = v.into_iter();

    let reader = casync_format::ChunkReader::new(|| {
        Ok(match it.next() {
            Some(chunk) => Some(chunk.open_from(castr)?),
            None => None,
        })
    }).chain_err(|| "initialising reader")?;

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    casync_format::read_stream(reader, |path, entry, data| {
        println!("{}, {:?}", utf8_path(path)?, entry);
        let mut buf = vec![];
        if data.is_some() {
            data.unwrap().read_to_end(&mut buf)?;
        }
        Ok(())
    }).chain_err(|| format!("reading stream of index {}", caidx))
}

fn utf8_path(from: &[Vec<u8>]) -> std::result::Result<String, ::std::string::FromUtf8Error> {
    let mut ret = String::new();
    for component in from {
        ret.push_str(String::from_utf8(component.clone())?.as_str());
        ret.push_str(" // ");
    }

    if !ret.is_empty() {
        let waste = ret.len() - 4;
        ret.truncate(waste);
    }

    Ok(ret)
}

quick_main!(run);
