use crate::lexer::Lexer;
use std::path::Path;
use std::sync::Mutex;
use std::{fs::File, sync::Arc};
use tiny_http::{Header, Method, Request, Response, StatusCode};

use crate::{idf, tf, Index};

pub fn start_server(index: Arc<Mutex<Index>>, address: String) {
    let server = tiny_http::Server::http(&address)
        .map_err(|e| {
            eprintln!("ERROR: Could not start server: {e}");
        })
        .unwrap();

    println!("Server started at http://{address}");

    for request in server.incoming_requests() {
        let _ = serve_request(Arc::clone(&index), request);
    }
}

pub fn serve_request(index: Arc<Mutex<Index>>, mut request: Request) -> Result<(), ()> {
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

            let index = index.lock().unwrap();

            for (path, doc) in &index.tfd {
                let (n, file_index) = (&doc.count, &doc.term_frequency);
                let mut score = 0.0;
                for term in Lexer::new(&body.chars().collect::<Vec<char>>()) {
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
