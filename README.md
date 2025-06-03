# oggmeta

[![Rust](https://github.com/hungl6844/oggmeta/actions/workflows/rust.yml/badge.svg)](https://github.com/hungl6844/oggmeta/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/oggmeta?label=latest)](https://crates.io/crates/oggmeta)
[![docs](https://docs.rs/oggmeta/badge.svg)](https://docs.rs/oggmeta/)

oggmeta is a crate that reads `.ogg` files and decodes the vorbis comments contained within.
### Features
oggmeta can read vorbis comments and theora comments, store any tags contained within, including album art.
in case the album art is contained as a theora frame, using `theorafile`, oggmeta can read the first frame of
video and store it as an album cover. 
### Limitations
regardless of how much video is in the file, oggmeta will always
assume there is only one frame of video and save the first one as an album cover.

additionally, oggmeta cannot write back tags to ogg files (yet, check the `dev` branch for progress).
