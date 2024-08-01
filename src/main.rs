use nanohtml2text::html2text;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs::*, usize};

struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn consume(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        token
    }

    fn consume_while<F>(&mut self, condition: F) -> &'a [char]
    where
        F: Fn(&char) -> bool,
    {
        let mut n = 0;
        while n < self.content.len() && condition(&self.content[n]) {
            n += 1;
        }
        self.consume(n)
    }

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.remove_whitespace();
        if self.content.len() == 0 {
            return None;
        }

        let c = self.content[0];

        if c.is_alphabetic() {
            return Some(self.consume_while(|c| c.is_alphabetic()));
        }

        if c.is_numeric() {
            return Some(self.consume_while(|c| c.is_numeric()));
        }

        return Some(self.consume(1));
    }

    fn remove_whitespace(&mut self) -> &'a [char] {
        while self.content.len() > 0 {
            let c = self.content[0];
            if c.is_whitespace() {
                self.content = &self.content[1..];
            } else {
                break;
            }
        }
        self.content
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn read_html_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file_content = read_to_string(file_path)?;
    Ok(html2text(&file_content))
}

type TermFrequency = HashMap<String, usize>;
type Index = HashMap<PathBuf, TermFrequency>;

fn serialize_index(index: &Index, index_path: &str) {
    println!("Serializing index to {:?}", index_path);
    let index_file = File::create(index_path).expect("Could not create index file");
    serde_json::to_writer(index_file, index).expect("Could not serialize index file");
}

fn deserialize_index(index_path: &str) -> Index {
    println!("Deserializing index from {:?}", index_path);
    let index_file = File::open(index_path).expect("Could not open index file");
    serde_json::from_reader(index_file).expect("Could not deserialize index file")
}

fn index_folder(folder_path: &str) -> io::Result<()> {

    let folder = read_dir(folder_path)?;

    let mut index: Index = HashMap::new();

    process_folder(folder, &mut index)?;

    let path = "index.json";
    serialize_index(&index, path);

    Ok(())
}

fn process_folder(folder: ReadDir, index: &mut Index) -> io::Result<()> {

    for entry in folder {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let subfolder = read_dir(&path)?;

            println!("Indexing folder {:?}", path);

            process_folder(subfolder, index)?;
        } else {

            println!("Indexing file {:?}", path);

            let file_content = read_html_file(&path)?.chars().collect::<Vec<char>>();

            let mut term_frequency: TermFrequency = HashMap::new();

            for token in Lexer::new(&file_content) {
                let id = token
                    .iter()
                    .map(|x| x.to_ascii_uppercase())
                    .collect::<String>();

                if let Some(count) = term_frequency.get_mut(&id) {
                    *count += 1;
                } else {
                    term_frequency.insert(id, 1);
                }
            }

            index.insert(path, term_frequency);
        }
    }

    Ok(())
}

fn check_index(index_path: &str) -> io::Result<()> {
    let index = deserialize_index(index_path);
    println!("{index_path} => {count} files", count = index.len());
    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = std::env::args();

    let command = args.nth(1).unwrap_or_else(|| {
        eprintln!("ERROR: No command provided");
        exit(1);
    });

    match command.as_str() {
        "index" => {
            let dir_path = args.next().unwrap_or_else(|| {
                eprintln!("ERROR: No directory provided");
                eprint!("USAGE: index <directory>");
                exit(1);
            });

            index_folder(&dir_path).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                exit(1);
            });
        }
        "search" => {
            let index_path = args.next().unwrap_or_else(|| {
                eprintln!("ERROR: No index file provided");
                eprintln!("USAGE: search <index>");
                exit(1);
            });

            check_index(&index_path).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                exit(1);
            });
        }
        _ => {
            eprintln!("ERROR: Unknown command: {}", command);
            exit(1);
        }
    }

    Ok(())
}
