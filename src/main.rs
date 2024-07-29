use nanohtml2text::html2text;
use std::io::Result;
use std::path::Path;
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

fn read_html_file<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let file_content = read_to_string(file_path)?;
    Ok(html2text(&file_content))
}
fn main() -> Result<()> {
    let file_path = "docs/index.html";
    let file_content = read_html_file(file_path)?.chars().collect::<Vec<char>>();

    for token in Lexer::new(&file_content) {
        let id = token.iter().map(|x| x.to_ascii_uppercase()).collect::<String>();
        println!("{id}");
    }

    Ok(())
}
