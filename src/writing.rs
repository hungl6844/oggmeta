use std::io::{Cursor, Read, Seek, Write};

use ogg::{PacketReader, PacketWriteEndInfo, PacketWriter};

use crate::Tag;

pub(crate) fn insert_comments<W: Write + Seek + Read>(
    rw: &mut W,
    tags: &Tag,
) -> Result<u64, crate::Error> {
    let pos = rw.seek(std::io::SeekFrom::Current(0))?;
    let mut buf = vec![];
    rw.read_to_end(&mut buf)?;
    rw.seek(std::io::SeekFrom::Start(pos))?;

    let mut packet_reader = PacketReader::new(Cursor::new(buf));
    let mut packet_writer = PacketWriter::new(&mut *rw);

    while let Some(p) = packet_reader.read_packet()? {
        let stream_serial = p.stream_serial();
        let last_in_page = p.last_in_page();
        let last_in_stream = p.last_in_stream();
        let absgp = p.absgp_page();
        let mut packet_data = vec![];

        // for the first, 3 is the packet type (mesage header) and the other 6 bytes spell "vorbis" in utf8
        // in the second,  the first byte specifies the message header, and the other 6 spell out "theora"
        if p.data.len() >= 7
            && (p.data[0..7] == [3, 118, 111, 114, 98, 105, 115])
                /*|| p.data[0..7] == [0x81, 0x74, 0x68, 0x65, 0x6F, 0x72, 0x61])*/
        {
            write_u32(&mut packet_data, tags.vendor.len() as u32)?;
            packet_data.write_all(tags.vendor.as_bytes())?;
            write_u32(&mut packet_data, tags.comments.len() as u32)?;

            for (key, values) in tags.comments.iter() {
                for val in values {
                    let out_string = key.to_string() + "=" + val;
                    write_u32(&mut packet_data, out_string.len() as u32)?;
                    packet_data.write_all(out_string.as_bytes())?;
                }
            }

            packet_data.write_all(&[1_u8])?;
        } else {
            packet_data = p.data;
        }

        packet_writer.write_packet(
            packet_data,
            stream_serial,
            if last_in_page {
                PacketWriteEndInfo::EndPage
            } else if last_in_stream {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::NormalPacket
            },
            absgp,
        )?;
    }

    Ok(rw.seek(std::io::SeekFrom::Current(0))?)
}

fn write_u32<W: Write>(writer: &mut W, i: u32) -> Result<(), crate::Error> {
    writer.write_all(&i.to_le_bytes())?;

    Ok(())
}
