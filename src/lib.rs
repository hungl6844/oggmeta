//! `oggmeta` is a crate for reading and writing audio metadata for ogg vorbis files

use base64::Engine;
use image::RgbImage;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;
use thiserror::Error;

use crate::utils::read_picture_block;

mod reading;
mod utils;
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
    #[error("No vorbis or theora comment packet found. oggmeta only supports vorbis and theora comments. your file may be malformed.")]
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
    #[error("there was an error parsing the ogg file. your file is most likely malformed.")]
    ParseError,
    /// wrapper around [`std::ffi::NulError`]
    /// this means something in theorafile returned null.
    #[error("{0}")]
    NullError(#[from] std::ffi::NulError),
    /// wrapper around [`image::error::ImageError`]
    #[error("{0}")]
    ImageError(#[from] image::error::ImageError),
    /// wrapper around [`std::str::Utf8Error`]
    #[error("{0}")]
    StrError(#[from] std::str::Utf8Error),
    /// wrapper around [`ogg::OggReadError`]
    #[error("{0}")]
    OggError(#[from] ogg::OggReadError),
    /// wrapper around [`base64::DecodeError`]
    #[error("{0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// A struct that contains all the available metadata in the file.
#[derive(Debug)]
pub struct Tag {
    pub vendor: String,
    pub comments: HashMap<String, String>,
    pub pictures: Vec<Picture>,
}

/// Implementation of FLAC picture block (is also ogg's recommended way to store album art).
/// For an encoded description of this struct, see [RFC9639](https://www.rfc-editor.org/rfc/rfc9639.html#name-picture)
#[derive(Debug)]
pub struct Picture {
    /// the type of picture: see [RFC9639](https://www.rfc-editor.org/rfc/rfc9639.html#table13)
    pub picture_type: PictureType,
    /// the media type as specified by [RFC2046](https://www.rfc-editor.org/rfc/rfc9639.html#RFC2046)
    /// essentially the MIME
    pub media_type: String,
    /// description string (often this is just Cover (front) or something)
    pub description: String,
    /// width of the picture
    pub width: u32,
    /// height of the picture
    pub height: u32,
    /// color depth of the picture in bits per pixel
    pub color_depth: u32,
    /// For indexed-color pictures (e.g., GIF), the number of colors used; 0 for non-indexed pictures
    pub number_colors: u32,
    /// the actual picture data
    pub data: Vec<u8>,
}

/// The type of picture, as specified by [RFC9639](https://www.rfc-editor.org/rfc/rfc9639.html#table13)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PictureType {
    Other = 0,
    PngIcon = 1,
    GeneralIcon = 2,
    FrontCover = 3,
    BackCover = 4,
    LinerNotesPage = 5,
    MediaLabel = 6,
    LeadArtist = 7,
    Artist = 8,
    Conductor = 9,
    Band = 10,
    Composer = 11,
    Lyricist = 12,
    RecordingLocation = 13,
    DuringRecording = 14,
    DuringPerformance = 15,
    MovieScreenCapture = 16,
    BrightColoredFish = 17,
    Illustration = 18,
    BandLogo = 19,
    PublisherLogo = 20,
}

impl TryFrom<u32> for PictureType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PictureType::Other),
            1 => Ok(PictureType::PngIcon),
            2 => Ok(PictureType::GeneralIcon),
            3 => Ok(PictureType::FrontCover),
            4 => Ok(PictureType::BackCover),
            5 => Ok(PictureType::LinerNotesPage),
            6 => Ok(PictureType::MediaLabel),
            7 => Ok(PictureType::LeadArtist),
            8 => Ok(PictureType::Artist),
            9 => Ok(PictureType::Conductor),
            10 => Ok(PictureType::Band),
            11 => Ok(PictureType::Composer),
            12 => Ok(PictureType::Lyricist),
            13 => Ok(PictureType::RecordingLocation),
            14 => Ok(PictureType::DuringRecording),
            15 => Ok(PictureType::DuringPerformance),
            16 => Ok(PictureType::MovieScreenCapture),
            17 => Ok(PictureType::BrightColoredFish),
            18 => Ok(PictureType::Illustration),
            19 => Ok(PictureType::BandLogo),
            20 => Ok(PictureType::PublisherLogo),
            _ => Err(()),
        }
    }
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
        reading::parse_file(read)
    }

    /// This function does the same as [`read_from`](crate::Tag::read_from), but takes a path instead, opening a [`File`]
    ///
    /// # Errors
    /// see [`read_from`](crate::Tag::read_from)
    pub fn read_from_path<P: AsRef<Path>>(path: &P) -> Result<Tag, Error> {
        let mut file = File::open(path)?;

        reading::parse_file(&mut file)
    }

    /// takes a [`Read`] and a [`Write`], copies the packets over.
    /// edits the vorbis comment header only.
    pub fn write_to<W: Write, R: Read + Seek>(
        &mut self,
        read: &mut R,
        write: &mut W,
    ) -> Result<(), crate::Error> {
        crate::writing::insert_comments(read, write, self)?;
        Ok(())
    }

    /// does the same thing as [`Tag::write_to`], but takes a path instead.
    pub fn write_to_path<P: AsRef<Path>>(&mut self, path: &P) -> Result<(), crate::Error> {
        let mut read = File::options()
            .read(true)
            .write(false)
            .create(false)
            .open(path)?;

        let mut write = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.as_ref().with_extension(".ogg.temp"))?;

        self.write_to(&mut read, &mut write)?;

        std::fs::remove_file(path)?;
        std::fs::rename(path.as_ref().with_extension(".ogg.temp"), path)?;

        Ok(())
    }
}

impl Picture {
    pub fn from_raw_block(data: &Vec<u8>) -> Result<Picture, crate::Error> {
        let mut buf = base64::engine::general_purpose::STANDARD_NO_PAD.decode(data)?;

        read_picture_block(&mut Cursor::new(&mut buf))
    }
}

impl From<RgbImage> for Picture {
    fn from(img: RgbImage) -> Self {
        let mut img_buf = Cursor::new(vec![]);
        let (width, height) = img.dimensions();
        img.write_to(&mut img_buf, image::ImageFormat::Jpeg)
            .unwrap();

        Picture {
            picture_type: PictureType::FrontCover,
            media_type: "image/jpeg".to_string(),
            description: "Cover (front)".to_string(),
            width,
            height,
            color_depth: 24,
            number_colors: 0,
            data: img_buf.into_inner(),
        }
    }
}
