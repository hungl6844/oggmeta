//! `oggmeta` is a crate for reading and writing audio metadata for ogg vorbis files

use std::collections::HashMap;
use std::convert::AsRef;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use thiserror::Error;

mod reading;
mod writing;

const VORBIS_HEADER: [u8; 7] = [3, 118, 111, 114, 98, 105, 115];
const THEORA_HEADER: [u8; 7] = [0x81, 0x74, 0x68, 0x65, 0x6F, 0x72, 0x61];

/// Error type.
///
/// An enum that contains the possible errors this crate can throw.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// no comment packet was found in the file. this suggests the ogg file is malformed or
    /// uses a codec besides vorbis/theora.
    #[error("No vorbis or theora comment packet found. oggmeta only supports vorbis and theora comments")]
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
    /// there was some error while reading the ogg file. this generally suggests
    /// some kind of error with libogg, libtheora, or theorafile.
    #[error("there was an error parsing the ogg file.")]
    ParseError,
    /// wrapper around [`std::ffi::NulError`]
    /// this means something in theorafile returned null.
    #[error("{0}")]
    NullError(#[from] std::ffi::NulError),
    /// wrapper around [`image::error::ImageError`]
    #[error("{0}")]
    ImageError(#[from] image::error::ImageError),
    #[error("{0}")]
    OggError(#[from] ogg::OggReadError),
}

/// A struct that contains all the available metadata in the file.
#[derive(Debug)]
pub struct Tag {
    pub vendor: String,
    pub comments: HashMap<String, Vec<String>>,
}

impl Tag {
    /// attempts to read metadata from a [`Read`], returning a [`Tag`]
    ///
    /// # Errors
    /// This function will error if the ogg file is malformed, or if it does not contain a type 3
    /// vorbis packet (the packet that contains the metadata.)
    ///
    /// This function could also error if the architecture of the target causes [`usize`] to be
    /// unable to contain a [`u32`] (below 32-bit, very unlikely)
    ///
    /// Lastly, this function will error if a non-utf8 character is contained in the packet, which
    /// goes against the vorbis specification.
    pub fn read_from<R: Read + Seek>(read: &mut R) -> Result<Tag, Error> {
        let (vendor, comments) = reading::parse_file(read)?;

        Ok(Tag { vendor, comments })
    }

    /// This function does the same as [`read_from`](crate::Tag::read_from), but takes a path instead, opening a [`File`]
    ///
    /// # Errors
    /// see [`read_from`](crate::Tag::read_from)
    pub fn read_from_path<P: AsRef<Path>>(path: &P) -> Result<Tag, Error> {
        let mut file = File::open(path)?;
        let (vendor, comments) = reading::parse_file(&mut file)?;

        Ok(Tag { vendor, comments })
    }

    /// takes a [`Read`] and a [`Write`], copies the packets over.
    /// edits the vorbis comment header only.
    pub fn write_to<W: Write, R: Read + Seek>(
        &self,
        read: &mut R,
        write: &mut W,
    ) -> Result<(), crate::Error> {
        crate::writing::insert_comments(read, write, self)
    }

    /// does the same thing as [`Tag::write_to`], but takes a path instead.<br>
    /// at some point i can consider deleting the previous file... keeping it for debug purposes.
    pub fn write_to_path<P: AsRef<Path>>(
        &self,
        read_from: &P,
        write_from: &P,
    ) -> Result<(), crate::Error> {
        let mut read = File::options()
            .read(true)
            .write(false)
            .create(false)
            .open(read_from)?;

        let mut write = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(write_from)?;

        self.write_to(&mut read, &mut write)?;

        Ok(())
    }
}
