use oggmeta::Tag;

fn main() {
    let mut tags = Tag::read_from_path(&"Laufey - Tough Luck.ogg").unwrap();
    tags.comments.remove("COVERART");
    dbg!(&tags);
    tags.comments
        .insert("title".to_string(), vec!["Good Luck".to_string()]);
    tags.write_to_path(&"Laufey - Tough Luck.ogg").unwrap();
}
