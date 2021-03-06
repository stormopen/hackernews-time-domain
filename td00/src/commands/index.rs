use std::fs::{create_dir, remove_dir_all, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

use std::string::String;

use clap::ArgMatches;

use crossbeam::crossbeam_channel::{unbounded, Receiver};
use serde::{Deserialize, Serialize};

use crate::TANINDEX;
use tantivy::schema::Field;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter};

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    id: u64,
    title: String,
}

fn add_my_doc(
    index_writer: &mut IndexWriter,
    field_id: Field,
    field_title: Field,
    id: u64,
    title: &str,
) {
    let doc = doc!(field_title => title, field_id => id);
    index_writer.add_document(doc);
}

fn create_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_u64_field("id", FAST | STORED);
    schema_builder.build()
}

fn create_index() -> tantivy::Result<Index> {
    let schema = create_schema();

    let check_path = Path::new(TANINDEX);
    let dir_exists = check_path.exists();
    if dir_exists {
        remove_dir_all(check_path).expect("dir does not exist");
    }

    let index_path = Path::new(TANINDEX);
    create_dir(index_path).expect("dir already exists");

    let index = Index::create_in_dir(&index_path, schema.clone())?;

    Ok(index)
}

fn process_lines(
    r: Receiver<String>,
    mut index_writer: &mut IndexWriter,
    field_id: Field,
    field_title: Field,
) {
    let item_json = r.recv().unwrap();

    let item: Item = serde_json::from_str(&item_json).unwrap();
    let id = &item.id;
    let title = &item.title;

    add_my_doc(&mut index_writer, field_id, field_title, *id, title);

    // println!("{} {}", id, title);
}

fn read_file_to_buffer(filename: String) -> tantivy::Result<()> {
    let index = create_index().unwrap();

    let id: Field = index.schema().get_field("id").unwrap();
    let title: Field = index.schema().get_field("title").unwrap();

    let mut index_writer = index.writer_with_num_threads(1, 3_000_000)?;

    index_writer.add_document(doc!(title => "The Diary of Muadib", id => 1u64));
    index_writer.add_document(doc!(title => "A Dairy Cow", id => 10u64));

    let f = File::open(filename).unwrap();
    let file = BufReader::new(&f);

    for (_num, line) in file.lines().enumerate() {
        // Create a channel of unbounded capacity.
        let (s, r) = unbounded();

        let l = line.unwrap();
        // Send a message into the channel.
        s.send(l).unwrap();

        // add_my_doc(&mut index_writer, id, title, 123u64, "Rock and Roll");

        process_lines(r, &mut index_writer, id, title);
    }

    index_writer.commit()?;
    Ok(())
}

pub fn run_index_cli(argmatch: &ArgMatches) -> Result<(), String> {
    let pb = PathBuf::from(argmatch.value_of("file").unwrap());

    let filename = pb.to_str().unwrap().to_string();

    index_file(filename).map_err(|e| format!("Indexing file failed : {:?}", e))
}

fn index_file(filename: String) -> tantivy::Result<()> {
    let _contents = read_file_to_buffer(filename.to_string());
    Ok(())
}
