use std::io::{Read, Write};

use base64::Engine;

use crate::{Picture, PictureType};

pub(crate) fn yuv444_to_rgb(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
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

pub(crate) fn write_u32<W: Write>(writer: &mut W, i: u32) -> Result<(), crate::Error> {
    let buf = &i.to_ne_bytes();
    writer.write_all(buf)?;

    Ok(())
}

fn write_u32_be<W: Write>(writer: &mut W, i: u32) -> Result<(), crate::Error> {
    let buf = &i.to_be_bytes();
    writer.write_all(buf)?;

    Ok(())
}

pub(crate) fn read_u32_be<R: Read>(reader: &mut R) -> Result<u32, crate::Error> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub(crate) fn create_picture_block(img: &mut Picture) -> Result<Vec<u8>, crate::Error> {
    let mut block = vec![];

    // we can parse ffmpeg's titles later, for now I'm assuming its always a front cover
    write_u32_be(&mut block, img.picture_type as u32)?;
    write_u32_be(&mut block, img.media_type.len() as u32)?;
    block.append(&mut img.media_type.as_bytes().to_vec());
    write_u32_be(&mut block, img.description.len() as u32)?;
    block.append(&mut img.description.as_bytes().to_vec());
    write_u32_be(&mut block, img.width)?;
    write_u32_be(&mut block, img.height)?;
    write_u32_be(&mut block, img.color_depth)?;
    write_u32_be(&mut block, img.number_colors)?;
    write_u32_be(&mut block, img.data.len() as u32)?;

    block.append(&mut img.data);

    let mut out = "METADATA_BLOCK_PICTURE=".as_bytes().to_vec();
    out.append(
        &mut base64::engine::general_purpose::STANDARD_NO_PAD
            .encode(block)
            .as_bytes()
            .to_vec(),
    );

    Ok(out)
}

pub(crate) fn read_picture_block<R: Read>(reader: &mut R) -> Result<Picture, crate::Error> {
    let picture_type = read_u32_be(reader)?;

    let media_type_len = read_u32_be(reader)? as usize;
    let mut media_type_buf = vec![0; media_type_len];
    reader.read_exact(&mut media_type_buf)?;
    let media_type = String::from_utf8(media_type_buf)?;

    let description_len = read_u32_be(reader)? as usize;
    let mut description_buf = vec![0; description_len];
    reader.read_exact(&mut description_buf)?;
    let description = String::from_utf8(description_buf)?;

    let width = read_u32_be(reader)?;
    let height = read_u32_be(reader)?;
    let color_depth = read_u32_be(reader)?;
    let number_colors = read_u32_be(reader)?;

    let data_len = read_u32_be(reader)? as usize;
    let mut data = vec![0; data_len];
    reader.read_exact(&mut data)?;

    Ok(Picture {
        picture_type: picture_type.try_into().unwrap_or(PictureType::FrontCover),
        media_type,
        description,
        width,
        height,
        color_depth,
        number_colors,
        data,
    })
}
