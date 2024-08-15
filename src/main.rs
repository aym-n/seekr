use nanohtml2text::html2text;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::SystemTime;
use std::{fs::*, usize};
use std::{io, thread};

use std::sync::{Arc, Mutex};

mod lexer;
use crate::lexer::*;

mod server;
use crate::server::*;

type TermFrequency = HashMap<String, usize>;
type TermFrequencyPerDoc = HashMap<PathBuf, Doc>;
type DocFrequency = HashMap<String, usize>;

#[derive(Serialize, Deserialize)]
struct Doc {
    term_frequency: TermFrequency,
    count: usize,
    last_modified: SystemTime,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Index {
    tfd: TermFrequencyPerDoc,
    df: DocFrequency,
}

fn read_html_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file_content = read_to_string(file_path)?;
    Ok(html2text(&file_content))
}

fn serialize_index(index: &Index, index_path: &PathBuf) {
    println!("Serializing index to {:?}", index_path);
    let index_file = File::create(index_path).expect("Could not create index file");
    serde_json::to_writer(index_file, index).expect("Could not serialize index file");
}

fn deserialize_index(index_path: &PathBuf) -> Index {
    println!("Deserializing index from {:?}", index_path);
    let index_file = File::open(index_path).expect("Could not open index file");
    serde_json::from_reader(index_file).expect("Could not deserialize index file")
}

fn index_folder(folder_path: &str, index_path: &PathBuf) -> io::Result<()> {
    let folder = read_dir(folder_path)?;

    let mut index = Default::default();

    process_folder(folder, &mut index)?;

    let index = index.lock().unwrap();
    serialize_index(&index, index_path);

    Ok(())
}

fn process_folder(folder: ReadDir, index: &Arc<Mutex<Index>>) -> io::Result<()> {
    for entry in folder {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let subfolder = read_dir(&path)?;
            println!("Indexing folder {:?}", path);
            process_folder(subfolder, index)?;
        } else {
            if path.extension().unwrap_or_default() != "html" {
                println!("INFO: Skipping file {:?}", path);
                continue;
            }
            let mut index = index.lock().unwrap();
            if let Some(doc) = index.tfd.get(&path) {
                let last_modified = path.metadata()?.modified()?;
                if doc.last_modified >= last_modified {
                    println!("INFO: Skipping file {:?}", path);
                    continue;
                } else {
                    println!("INFO: Updating file {:?}", path);
                    remove_file_from_index(&mut index, &path)?;
                    add_file_to_index(path, &mut index)?;
                }
            } else {
                println!("INFO: Indexing file {:?}", path);
                add_file_to_index(path, &mut index)?;
            }
        }
    }

    Ok(())
}

fn remove_file_from_index(index: &mut Index, file_path: &PathBuf) -> io::Result<()> {
    if let Some(doc) = index.tfd.remove(file_path) {
        for (term, _) in doc.term_frequency {
            if let Some(count) = index.df.get_mut(&term) {
                *count -= 1;
            }
        }
    }
    Ok(())
}

fn add_file_to_index(file_path: PathBuf, index: &mut Index) -> io::Result<()> {
    let last_modified = file_path.metadata()?.modified()?;

    let file_content = read_html_file(&file_path)?.chars().collect::<Vec<char>>();

    let mut term_frequency: TermFrequency = HashMap::new();
    let mut n = 0;
    for token in Lexer::new(&file_content) {
        if let Some(count) = term_frequency.get_mut(&token) {
            *count += 1;
        } else {
            term_frequency.insert(token, 1);
        }
        n += 1;
    }

    for token in term_frequency.keys() {
        if let Some(count) = index.df.get_mut(token) {
            *count += 1;
        } else {
            index.df.insert(token.to_string(), 1);
        }
    }

    index.tfd.insert(file_path.to_path_buf(), {
        Doc {
            term_frequency,
            count: n,
            last_modified,
        }
    });

    Ok(())
}

pub fn tf(index: &TermFrequency, n: usize, term: &str) -> f32 {
    index.get(term).cloned().unwrap_or(0) as f32 / n as f32
}

pub fn idf(df: &DocFrequency, n: usize, term: &str) -> f32 {
    let n = n as f32;
    let m = df.get(term).cloned().unwrap_or(1) as f32;
    (n / m).log(2.0)
}

fn main() -> io::Result<()> {
    let mut args = std::env::args();

    let command = args.nth(1).unwrap_or_else(|| {
        eprintln!("ERROR: No command provided");
        exit(1);
    });

    match command.as_str() {
        // "index" => {
        //     let dir_path = args.next().unwrap_or_else(|| {
        //         eprintln!("ERROR: No directory provided");
        //         eprint!("USAGE: index <directory>");
        //         exit(1);
        //     });

        //     let index_path = Path::new(&dir_path).with_extension("json");

        //     index_folder(&dir_path, &index_path).unwrap_or_else(|e| {
        //         eprintln!("ERROR: {}", e);
        //         exit(1);
        //     });
        // }

        // "reindex" => {
        //     let dir_path = args.next().unwrap_or_else(|| {
        //         eprintln!("ERROR: No directory provided");
        //         eprint!("USAGE: reindex <directory>");
        //         exit(1);
        //     });

        //     let index_path = Path::new(&dir_path).with_extension("json");

        //     let mut index = deserialize_index(&index_path);

        //     let folder = read_dir(&dir_path).unwrap_or_else(|e| {
        //         eprintln!("ERROR: {}", e);
        //         exit(1);
        //     });

        //     process_folder(folder, &mut index).unwrap_or_else(|e| {
        //         eprintln!("ERROR: {}", e);
        //         exit(1);
        //     });

        //     serialize_index(&index, &index_path);
        // }
        "serve" => {
            let folder_name = args.next().unwrap_or_else(|| {
                eprintln!("ERROR: No directory provided");
                eprint!("USAGE: serve <directory>");
                exit(1);
            });

            let index_path = Path::new(&folder_name).with_extension("json");

            let index = if index_path.exists() {
                Arc::new(Mutex::new(deserialize_index(&index_path)))
            } else {
                Default::default()
            };

            {
                let index = Arc::clone(&index);
                thread::spawn(move || {
                    let index = Arc::clone(&index);
                    process_folder(read_dir(folder_name).unwrap(), &index)
                        .map_err(|e| {
                            eprintln!("ERROR: {}", e);
                            exit(1);
                        })
                        .unwrap();

                    let index = index.lock().unwrap();
                    serialize_index(&index, &index_path);
                });
            }

            start_server(Arc::clone(&index), "127.0.0.1:8000".to_string());
        }

        _ => {
            eprintln!("ERROR: Unknown command: {}", command);
            exit(1);
        }
    }

    Ok(())
}
