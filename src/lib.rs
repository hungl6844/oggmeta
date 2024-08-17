//! `oggmeta` is a crate for reading (and soon writing) audio metadata for ogg vorbis files

use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Seek};
use std::path::Path;
use thiserror::Error;

mod ogg;

/// Error type.
///
/// An enum that contains the possible errors this crate can throw.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// no comment packet was found in the file. this suggests the ogg file is malformed or
    /// uses a codec besides vorbis.
    #[error("No vorbis comment packet found. oggmeta only supports vorbis")]
    NoComments,
    /// wrapper around [`std::io::Error`]. generally caused by problems reading the file.
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    /// wrapper around [`std::string::FromUtf8Error`]. this means that your message vector contains
    /// malformed UTF-8
    #[error("{0}")]
    InvalidString(#[from] std::string::FromUtf8Error),
    /// wrapper around [`std::num::TryFromIntError`]. This means that one of the string indexes in
    /// the file are unvalid [`u32`]s
    #[error("{0}")]
    InvalidLength(#[from] std::num::TryFromIntError),
}

/// A struct that contains all the available metadata in the file.
#[derive(Debug)]
pub struct Tag {
    vendor: String,
    comments: HashMap<String, Vec<String>>,
}

impl Tag {
    /// attempts to read metadata from a [`Read`], returning a [`Tag`]
    ///
    /// # Errors
    /// This function will error if the ogg file is malformed, or if it does not contain a type 3
    /// vorbis packet (the packet that contains the metadata.)
    ///
    /// This function could also error if the architecture of the target causes the [`usize`] to be
    /// unable to contain a [`u32`] (below 32-bit, very unlikely)
    ///
    /// Lastly, this function will error if a non-utf8 character is contained in the packet, which
    /// goes against the vorbis specification.
    pub fn read_from<R: Read + Seek>(read: &mut R) -> Result<Tag, Error> {
        let (vendor, comments) = ogg::parse_file(read)?;

        Ok(Tag { vendor, comments })
    }

    /// This function does the same as `read_from`, but takes a path instead, opening a [`File`]
    ///
    /// # Errors
    /// see `read_from`
    pub fn read_from_path<P: AsRef<Path>>(path: &P) -> Result<Tag, Error> {
        let mut file = File::open(path)?;
        let (vendor, comments) = ogg::parse_file(&mut file)?;

        Ok(Tag { vendor, comments })
    }

    /// This function returns the vendor of the music file.
    pub fn get_vendor(&self) -> String {
        self.vendor.clone()
    }

    /// This function fetches a comment and returns `None` if the key does not exist in the file.
    pub fn get<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<Vec<String>>
    where
        String: Borrow<Q>,
    {
        self.comments.get(key).cloned()
    }
}
