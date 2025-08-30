use oggmeta::Tag;

fn main() {
    let mut tags = Tag::read_from_path(&"Creep (Original).ogg").or_else(|e| => { dbg!(e) });

    tags.comments.remove("COVERART");

    dbg!(&tags);

    tags.comments
        .insert("title".to_string(), vec!["hello".to_string()]);

    tags.write_to_path(&"Creep (Original).ogg", &"Creep.ogg")
        .unwrap();
}
