use thiserror::Error;
use std::collections::HashMap;
use std::io::{Seek, Read};
use std::path::Path;
use std::fs::File;
use std::convert::AsRef;

mod ogg;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("No vorbis comment packet found. oggmeta only supports vorbis")]
    NoComments,
    #[error("{0}")]
    InvalidString(#[from] std::string::FromUtf8Error),
    #[error("{0}")]
    InvalidLength(#[from] std::num::TryFromIntError),
}

#[derive(Debug)]
pub struct Tag {
    vendor: String,
    comments: HashMap<String, Vec<String>>
}

impl Tag {
    pub fn read_from<R: Read + Seek> (read: &mut R) -> Result<Tag, Error> {
        let (vendor, comments) = ogg::parse_file(read)?;

        Ok(Tag {vendor, comments})
    }

    pub fn read_from_path<P: AsRef<Path>> (path: &P) -> Result<Tag, Error> {
        let mut file = File::open(path)?;
        let (vendor, comments) = ogg::parse_file(&mut file)?;

        Ok(Tag {vendor, comments})
    }

    pub fn get_vendor(&self) -> String { self.vendor.clone() }
}
