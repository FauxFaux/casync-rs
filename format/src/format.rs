use crate::errors::*;

const ENTRY: u64 = 0x1396fabcea5bbb51;
const USER: u64 = 0xf453131aaeeaccb3;
const GROUP: u64 = 0x25eb6ac969396a52;
const FILENAME: u64 = 0x6dbb6ebcb3161f0b;
const PAYLOAD: u64 = 0x8b9e1d93d6dcffc9;
const GOODBYE: u64 = 0xdfd35c5e8327c403;

const INDEX: u64 = 0x96824d9c7b129ff9;
const TABLE: u64 = 0xe75b9e112f17417d;

pub type ChunkId = [u8; 32];

#[derive(Eq, PartialEq, Debug)]
pub enum StreamMagic {
    Entry,
    User,
    Group,
    Name,
    Data,
    Bye,
}

#[derive(Eq, PartialEq, Debug)]
pub enum IndexMagic {
    Index,
    Table,
}

impl StreamMagic {
    pub fn from(val: u64) -> Result<Self> {
        use self::StreamMagic::*;
        Ok(match val {
            ENTRY => Entry,
            USER => User,
            GROUP => Group,
            FILENAME => Name,
            PAYLOAD => Data,
            GOODBYE => Bye,
            _ => bail!("unrecognised stream magic: {:x}", val),
        })
    }
}

impl IndexMagic {
    pub fn from(val: u64) -> Result<Self> {
        use self::IndexMagic::*;
        Ok(match val {
            INDEX => Index,
            TABLE => Table,
            _ => bail!("unrecognised index magic: {:x}", val),
        })
    }
}
