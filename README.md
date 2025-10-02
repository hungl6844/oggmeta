# oggmeta

[![Rust](https://github.com/hungl6844/oggmeta/actions/workflows/rust.yml/badge.svg)](https://github.com/hungl6844/oggmeta/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/oggmeta?label=latest)](https://crates.io/crates/oggmeta)
[![docs](https://docs.rs/oggmeta/badge.svg)](https://docs.rs/oggmeta/)

oggmeta is a crate that reads `.ogg` files and decodes the vorbis comments contained within.
### Features
oggmeta can read and write vorbis comments and theora comments within ogg files

in case the album art is contained as a theora frame, using `theorafile`, oggmeta can read the first frame of
video and store it as an album cover. 

NOTE: that implementation is pretty much completely wrong. ffmpeg does this by default when converting from, for instance, an mp3, but covert art in an ogg file should _always_ be stored in
`METADATA_BLOCK_IMAGE`. However, I don't want oggmeta to overwrite data in an ogg file (besides tags);
oggmeta will keep the theora stream, but will exclusively write album covers to 
`METADATA_BLOCK_IMAGE` tag.

### Limitations
regardless of how much video is in the file, oggmeta will always
assume there is only one frame of video and save the first one as an album cover.
