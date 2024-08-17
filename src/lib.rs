use thiserror::Error;

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

#[cfg(test)]
mod tests {
    use crate::ogg;
    use std::fs::File;

    #[test]
    fn it_works() {
        let mut ogg = File::open("audio.ogg").unwrap();
        ogg::get_comments(&mut ogg).unwrap();
    }
}
