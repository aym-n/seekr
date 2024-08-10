use nanohtml2text::html2text;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::SystemTime;
use std::{fs::*, usize};

use rust_stemmers::{Algorithm, Stemmer};

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

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

    fn next_token(&mut self) -> Option<String> {
        self.remove_whitespace();
        if self.content.len() == 0 {
            return None;
        }

        let c = self.content[0];

        if c.is_alphabetic() {
            let token = self
                .consume_while(|c| c.is_alphabetic())
                .iter()
                .map(|x| x.to_ascii_lowercase())
                .collect::<String>();
            let stemmer = Stemmer::create(Algorithm::English);
            let stemmed_token = stemmer.stem(&token);
            return Some(stemmed_token.to_ascii_uppercase());
        }

        if c.is_numeric() {
            return Some(self.consume_while(|c| c.is_numeric()).iter().collect());
        }

        return Some(self.consume(1).iter().collect());
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
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn read_html_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file_content = read_to_string(file_path)?;
    Ok(html2text(&file_content))
}

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
struct Index {
    tfd: TermFrequencyPerDoc,
    df: DocFrequency,
}

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

    let mut index: Index = Default::default();

    process_folder(folder, &mut index)?;

    let path = "index.json";
    serialize_index(&index, path);

    Ok(())
}

fn process_folder(folder: ReadDir, index: &mut Index) -> io::Result<()> {
    for entry in folder {
        let entry = entry?;
        let path = entry.path();
        
        let last_modified = path.metadata()?.modified()?;

        if path.is_dir() {
            let subfolder = read_dir(&path)?;

            println!("Indexing folder {:?}", path);

            process_folder(subfolder, index)?;
        } else {
            println!("Indexing file {:?}", path);

            let file_content = read_html_file(&path)?.chars().collect::<Vec<char>>();

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

            index.tfd.insert(path, {
                Doc {
                    term_frequency,
                    count: n,
                    last_modified,
                }
            });
        }
    }

    Ok(())
}

fn check_index(index_path: &str) -> io::Result<()> {
    let index = deserialize_index(index_path);
    println!("{index_path} => {count} files", count = index.tfd.len());
    Ok(())
}

fn serve_static_files(request: Request, path: &str, content_type: &str) -> Result<(), ()> {
    let header = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap();
    let file = File::open(path).map_err(|e| {
        eprintln!("ERROR: Could not open file: {e}");
    })?;

    let response = Response::from_file(file)
        .with_header(header)
        .with_status_code(StatusCode(200));
    request.respond(response).map_err(|e| {
        eprintln!("ERROR: Could not respond to request: {e}");
    })?;
    Ok(())
}

fn tf(index: &TermFrequency, n: usize, term: &str) -> f32 {
    index.get(term).cloned().unwrap_or(0) as f32 / n as f32
}

fn idf(df: &DocFrequency, n: usize, term: &str) -> f32 {
    let n = n as f32;
    let m = df.get(term).cloned().unwrap_or(1) as f32;
    (n / m).log(2.0)
}

fn serve_request(index: &Index, mut request: Request) -> Result<(), ()> {
    println!(
        "INFO: Incoming request, method: {:?}, url: {}",
        request.method(),
        request.url()
    );

    match request.method() {
        Method::Post => {
            let mut body = Vec::new();
            request.as_reader().read_to_end(&mut body).map_err(|e| {
                eprintln!("ERROR: Could not read request body: {e}");
            })?;

            let body = String::from_utf8(body).map_err(|e| {
                eprintln!("ERROR: Could not parse request body: {e}");
            })?;

            let mut result = Vec::<(&Path, f32)>::new();

            for (path, doc) in &index.tfd {
                let (n, file_index) = (&doc.count, &doc.term_frequency);
                let mut score = 0.0;
                for term in Lexer::new(&body.chars().collect::<Vec<char>>()) {
                    print!("{:?} ", term);
                    score += tf(&file_index, *n, &term) * idf(&index.df, index.tfd.len(), &term);
                }
                result.push((path, score));
            }

            result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            for (path, score) in result.iter().take(10) {
                println!("{:?} => {}", path, score);
            }

            let result_json = serde_json::to_string(&result).map_err(|e| {
                eprintln!("ERROR: Could not serialize response: {e}");
            })?;

            let response = Response::from_string(result_json)
                .with_header(Header::from_bytes(&b"Content-Type"[..], b"application/json").unwrap())
                .with_status_code(StatusCode(200));

            request.respond(response).map_err(|e| {
                eprintln!("ERROR: Could not respond to request: {e}");
            })?;
        }

        Method::Get => match request.url() {
            "/" | "/index.html" => {
                serve_static_files(request, "static/index.html", "text/html")?;
            }

            "/index.js" => {
                serve_static_files(request, "static/index.js", "application/javascript")?;
            }

            _ => {
                serve_static_files(request, "static/404.html", "text/html")?;
            }
        },

        _ => {
            serve_static_files(request, "static/404.html", "text/html")?;
        }
    }
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
        "serve" => {
            let index_path = args.next().unwrap_or("index.json".to_string());

            let index = deserialize_index(&index_path);

            let address = args.next().unwrap_or("0.0.0.0:8000".to_string());
            let server = Server::http(&address).unwrap_or_else(|err| {
                eprintln!("ERROR: Could not start server: {err}");
                exit(1);
            });

            println!("Server started at http://{address}");

            for request in server.incoming_requests() {
                let _ = serve_request(&index, request);
            }
        }
        _ => {
            eprintln!("ERROR: Unknown command: {}", command);
            exit(1);
        }
    }

    Ok(())
}
