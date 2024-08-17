use ogg::reading::PacketReader;
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
};

pub fn parse_file<T>(reader: &mut T) -> Result<(String, HashMap<String, Vec<String>>), crate::Error>
where
    T: Read + Seek,
{
    let mut packet_reader = PacketReader::new(reader);

    loop {
        let packet = packet_reader.read_packet();
        if let Ok(o) = packet {
            if let Some(p) = o {
                let mut packet_data = p.data;

                // 3 is the packet type (mesage header) and the other 6 bytes spell "vorbis" in utf8

                if packet_data[0..7] == [3, 118, 111, 114, 98, 105, 115] {
                    println!("{packet_data:?}");
                    let (vendor, comments) = parse_vorbis(&mut Cursor::new(&mut packet_data))?;
                    println!("{:?}", &comments);
                    return Ok((vendor, comments));
                }
            }
            return Err(crate::Error::NoComments);
        }
    }
}

pub fn parse_vorbis<T>(vorbis: &mut T) -> Result<(String, HashMap<String, Vec<String>>), crate::Error>
where
    T: Read + Seek,
{
    let mut comments: HashMap<String, Vec<String>> = HashMap::new();

    vorbis.seek(std::io::SeekFrom::Start(7))?;
    let vendor_length = read_u32(vorbis)?;
    println!("{}", vendor_length);
    let mut vendor_bytes = vec![0_u8; vendor_length.try_into()?];
    vorbis.read_exact(&mut vendor_bytes)?;
    let vendor_string = String::from_utf8(vendor_bytes)?;
    let list_length = read_u32(vorbis)?;

    for _x in 0..list_length {
        let length = read_u32(vorbis)?;
        let mut comment_bytes = vec![0_u8; length.try_into()?];
        vorbis.read_exact(&mut comment_bytes)?;
        let comment = String::from_utf8(comment_bytes)?;

        let mut split_comment = comment.split("=");
        comments
            .entry(split_comment.next().ok_or(crate::Error::NoComments)?.into())
            .or_default()
            .push(split_comment.next().ok_or(crate::Error::NoComments)?.into());
    }

    Ok((vendor_string, comments))
}

fn read_u8<T>(read: &mut T) -> Result<u8, crate::Error>
where
    T: Read,
{
    let mut buf = [0_u8; 1];
    read.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32<T>(read: &mut T) -> Result<u32, crate::Error>
where
    T: Read,
{
    let mut buf = [0_u8; 4];
    read.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}
