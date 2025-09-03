use oggmeta::Tag;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tags = Tag::read_from_path(&"Creep (Original).ogg")?;

    tags.comments.remove("COVERART");

    dbg!(&tags);

    tags.comments
        .insert("title".to_string(), vec!["hello".to_string()]);

    tags.write_to_path(&"Creep (Original).ogg", &"Creep.ogg")
        .unwrap();

    Ok(())
}
