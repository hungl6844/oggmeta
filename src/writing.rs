use image::EncodableLayout;
use ogg::{PacketReader, PacketWriteEndInfo, PacketWriter};
use std::io::{Read, Seek, Write};

use crate::{
    utils::{create_picture_block, write_u32},
    Tag, THEORA_HEADER, VORBIS_HEADER,
};

pub(crate) fn insert_comments<W: Write, R: Read + Seek>(
    read: &mut R,
    write: &mut W,
    tags: &mut Tag,
) -> Result<(), crate::Error> {
    let pos = read.stream_position()?;

    let mut packet_reader = PacketReader::new(&mut *read);
    let mut packet_writer = PacketWriter::new(write);

    while let Some(p) = packet_reader.read_packet()? {
        let stream_serial = p.stream_serial();
        let last_in_page = p.last_in_page();
        let last_in_stream = p.last_in_stream();
        let absgp = p.absgp_page();
        let mut packet_data: Vec<u8>;

        let is_vorbis = if p.data.len() >= 7 {
            p.data[0..7] == VORBIS_HEADER
        } else {
            false
        };
        let is_theora = if p.data.len() >= 7 {
            p.data[0..7] == THEORA_HEADER
        } else {
            false
        };

        if is_theora || is_vorbis {
            packet_data = if is_vorbis {
                VORBIS_HEADER.to_vec()
            } else {
                THEORA_HEADER.to_vec()
            };
            write_u32(&mut packet_data, tags.vendor.len() as u32)?;
            packet_data.write_all(tags.vendor.as_bytes())?;
            write_u32(
                &mut packet_data,
                tags.comments
                    .iter()
                    .map(|c| -> u32 { c.1.len() as u32 })
                    .sum::<u32>()
                    + tags.pictures.len() as u32,
            )?;

            for (key, vals) in tags.comments.iter() {
                for val in vals {
                    let out_string = key.to_string().to_uppercase() + "=" + val;
                    write_u32(&mut packet_data, out_string.len() as u32)?;
                    packet_data.write_all(out_string.as_bytes())?;
                }
            }

            if is_vorbis {
                for picture in &mut tags.pictures {
                    let block = create_picture_block(picture)?;
                    write_u32(&mut packet_data, (block.len()) as u32)?;
                    packet_data.write_all(block.as_bytes())?;
                }

                // this is the vorbis framing bit. not implemented in theora.
                packet_data.write_all(&[1_u8])?;
            };
        } else {
            packet_data = p.data;
        }

        packet_writer.write_packet(
            packet_data,
            stream_serial,
            if last_in_stream {
                PacketWriteEndInfo::EndStream
            } else if last_in_page {
                PacketWriteEndInfo::EndPage
            } else {
                PacketWriteEndInfo::NormalPacket
            },
            absgp,
        )?;
    }

    read.seek(std::io::SeekFrom::Start(pos))?;

    Ok(())
}
