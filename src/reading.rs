use base64::{prelude::BASE64_STANDARD, Engine};
use ogg::PacketReader;
use std::{
    alloc::Layout,
    collections::HashMap,
    io::{Cursor, Read, Seek},
};
use theorafile_rs::{
    ogg_int64_t, tf_callbacks, tf_close, tf_eos, tf_hasvideo, tf_open_callbacks, tf_readvideo,
    tf_videoinfo, th_pixel_fmt, th_pixel_fmt_TH_PF_444, OggTheora_File,
};

// this entire block for the DataSource struct (and most of my code for reading theora)
// is taken pretty much completely from `https://github.com/xmoezzz/omvdecoder`,
// this guy is the only example I could find of someone using the theorafile bindings,
// and he's also the person who made them. this guy actually saved my sanity.
struct DataSource {
    data: Vec<u8>,
    pos: usize,
}

impl DataSource {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    // Implement a method to seek to a specific position
    pub(crate) fn seek(
        &mut self,
        offset: ogg_int64_t,
        origin: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int {
        match origin {
            0 => self.pos = offset as usize,
            1 => self.pos = (self.pos as ogg_int64_t + offset) as usize,
            2 => self.pos = (self.data.len() as ogg_int64_t + offset) as usize,
            _ => return -1, // Unsupported origin
        }
        // Ensure pos doesn't go out of bounds
        if self.pos > self.data.len() {
            self.pos = self.data.len();
        }
        0 // Success
    }

    // Implement a method to read data from the current position
    pub(crate) fn read(
        &mut self,
        ptr: *mut ::std::os::raw::c_void,
        size: usize,
        nmemb: usize,
    ) -> usize {
        let bytes_to_read = size * nmemb;
        let remaining_data = &self.data[self.pos..];
        let bytes_read = std::cmp::min(remaining_data.len(), bytes_to_read);
        unsafe {
            std::ptr::copy_nonoverlapping(remaining_data.as_ptr(), ptr as *mut u8, bytes_read);
        }
        self.pos += bytes_read;
        bytes_read
    }

    // Implement a method to close the data source
    pub(crate) fn close(&mut self) -> ::std::os::raw::c_int {
        // Optionally perform any cleanup here
        0 // Success
    }
}

unsafe extern "C" fn read_func_impl(
    ptr: *mut ::std::os::raw::c_void,
    size: usize,
    nmemb: usize,
    datasource: *mut ::std::os::raw::c_void,
) -> usize {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.read(ptr, size, nmemb)
    } else {
        0
    }
}

unsafe extern "C" fn seek_func_impl(
    datasource: *mut ::std::os::raw::c_void,
    offset: ogg_int64_t,
    origin: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.seek(offset, origin)
    } else {
        -1
    }
}

unsafe extern "C" fn close_func_impl(
    datasource: *mut ::std::os::raw::c_void,
) -> ::std::os::raw::c_int {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.close()
    } else {
        -1
    }
}

pub(crate) fn parse_file<T>(
    reader: &mut T,
) -> Result<(String, HashMap<String, Vec<String>>), crate::Error>
where
    T: Read + Seek,
{
    let (vendor, mut tags) = parse_headers(reader)?;
    reader.rewind()?;

    let tf_cbs = tf_callbacks {
        read_func: Some(read_func_impl),
        seek_func: Some(seek_func_impl),
        close_func: Some(close_func_impl),
    };

    let mut ogg_data = vec![];
    reader.read_to_end(&mut ogg_data)?;
    let mut datasource = DataSource::new(ogg_data);

    let layout = Layout::new::<OggTheora_File>();
    let ogg = unsafe { std::alloc::alloc(layout) };
    if ogg.is_null() {
        return Err(crate::Error::ParseError);
    }
    let ogg_file = ogg as *mut OggTheora_File;

    let opened = unsafe {
        tf_open_callbacks(
            &datasource as *const DataSource as *mut DataSource as *mut std::os::raw::c_void,
            ogg_file,
            tf_cbs,
        )
    };
    if opened < 0 {
        if !ogg.is_null() {
            unsafe { std::alloc::dealloc(ogg, layout) };
        }
        return Err(crate::Error::ParseError);
    }

    let has_video = unsafe { tf_hasvideo(ogg_file) };

    if has_video == 1 {
        let mut width: ::std::os::raw::c_int = 0;
        let mut height: ::std::os::raw::c_int = 0;
        let mut fps: f64 = 0.0;
        let mut fmt: th_pixel_fmt = 0;

        unsafe { tf_videoinfo(ogg_file, &mut width, &mut height, &mut fps, &mut fmt) };

        if fmt != th_pixel_fmt_TH_PF_444 {
            unsafe { tf_close(ogg_file) };
            if !ogg.is_null() {
                unsafe { std::alloc::dealloc(ogg, layout) };
            }
            return Err(crate::Error::ParseError);
        }

        let size = width as usize * height as usize * 3;
        let alignment = 1024;
        let mem_layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };

        let data_blob = unsafe { std::alloc::alloc(mem_layout) };

        if data_blob.is_null() {
            unsafe { tf_close(ogg_file) };
            if !ogg.is_null() {
                unsafe { std::alloc::dealloc(ogg, layout) };
            }
            return Err(crate::Error::ParseError);
        }

        if unsafe { tf_eos(ogg_file) } == 0 {
            // if this doesn't return 0, theres... no video? even though we've established there is a video.
            // better safe than sorry, i guess.
            unsafe { tf_readvideo(ogg_file, data_blob, 1) };

            // need some kind of error checking here.
            // we can't run an unsafe function and just assume there's some data on the other side of the pointer.

            let mut img = image::RgbImage::new(width as u32, height as u32);

            let num_pixels = (width * height) as usize;
            let y_plane = unsafe { std::slice::from_raw_parts(data_blob, num_pixels) };
            let u_plane = unsafe {
                std::slice::from_raw_parts(data_blob.add(num_pixels), num_pixels as usize)
            };
            let v_plane = unsafe {
                std::slice::from_raw_parts(data_blob.add(2 * num_pixels), num_pixels as usize)
            };

            for y in 0..height {
                for x in 0..width {
                    let i = (y * width + x) as usize;

                    let y_val = y_plane[i] as f32;
                    let u_val = u_plane[i] as f32;
                    let v_val = v_plane[i] as f32;

                    let (r, g, b) = yuv444_to_rgb(y_val, u_val, v_val);

                    img.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
                }
            }

            let mut img_buf = Cursor::new(Vec::new());
            img.write_to(&mut img_buf, image::ImageFormat::Png)?;

            tags.insert(
                "COVERART".to_string(),
                vec![
                    "data:image/png;base64,".to_string()
                        + &BASE64_STANDARD.encode(img_buf.get_ref()),
                ],
            );

            unsafe {
                tf_close(ogg_file);

                if !ogg.is_null() {
                    std::alloc::dealloc(ogg, layout);
                }
                if !data_blob.is_null() {
                    std::alloc::dealloc(data_blob, mem_layout);
                }

                datasource.close();
            }
        }
    }

    Ok((vendor, tags))
}

fn parse_headers<T>(reader: &mut T) -> Result<(String, HashMap<String, Vec<String>>), crate::Error>
where
    T: Read + Seek,
{
    let mut packet_reader = PacketReader::new(reader);

    println!("reached!");

    if let Ok(mut r) = packet_reader.read_packet() {
        while let Some(ref mut p) = r {
            println!("loop!");

            // 3 is the packet type (mesage header) and the other 6 bytes spell "vorbis" in utf8

            if p.data.len() >= 7 && p.data[0..7] == [3, 118, 111, 114, 98, 105, 115] {
                let mut vorbis = Cursor::new(&mut p.data);
                let mut comments: HashMap<String, Vec<String>> = HashMap::new();

                vorbis.seek(std::io::SeekFrom::Start(7))?;
                let vendor_length = read_u32(&mut vorbis)?;
                let mut vendor_bytes = vec![0_u8; vendor_length.try_into()?];
                vorbis.read_exact(&mut vendor_bytes)?;
                let vendor_string = String::from_utf8(vendor_bytes)?;
                let list_length = read_u32(&mut vorbis)?;

                for _x in 0..list_length {
                    let length = read_u32(&mut vorbis)?;
                    let mut comment_bytes = vec![0_u8; length.try_into()?];
                    vorbis.read_exact(&mut comment_bytes)?;
                    let comment = String::from_utf8(comment_bytes)?;

                    let mut split_comment = comment.split("=");
                    comments
                        .entry(split_comment.next().ok_or(crate::Error::NoComments)?.into())
                        .or_default()
                        .push(split_comment.next().ok_or(crate::Error::NoComments)?.into());
                }

                return Ok((vendor_string, comments));
            }
        }

        // the packet reader failed. generally means the file is not a valid ogg file.
        return Err(crate::Error::NoComments);
    }

    Ok(("".to_string(), HashMap::new()))
}

fn read_u32<T>(read: &mut T) -> Result<u32, crate::Error>
where
    T: Read,
{
    let mut buf = [0_u8; 4];
    read.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn yuv444_to_rgb(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let b = (1.164 * (y - 16.0) + 2.018 * (u - 128.0))
        .clamp(0.0, 255.0)
        .round() as u8;
    let g = (1.164 * (y - 16.0) - 0.813 * (v - 128.0) - 0.391 * (u - 128.0))
        .clamp(0.0, 255.0)
        .round() as u8;
    let r = (1.164 * (y - 16.0) + 1.596 * (v - 128.0))
        .clamp(0.0, 255.0)
        .round() as u8;

    (r, g, b)
}
