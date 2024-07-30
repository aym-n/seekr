use nanohtml2text::html2text;
use std::collections::HashMap;
use std::io::Result;
use std::path::{Path, PathBuf};
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

type TermFrequency = HashMap<String, usize>;
type Index = HashMap<PathBuf, TermFrequency>;

fn main() -> Result<()> {

    let dir_path = "docs/char";
    let dir = read_dir(dir_path)?;

    let mut index: Index = HashMap::new();

    for file in dir {
        let file_path = file?.path();

        println!("Processing file: {:?}", &file_path);
        
        let file_content = read_html_file(&file_path)?.chars().collect::<Vec<char>>();
    
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
    
        let mut stats =  term_frequency.iter().collect::<Vec<_>>();
        stats.sort_by_key(|(_, f)| *f);
        stats.reverse();

        index.insert(file_path, term_frequency);

    }

    for (file_path, term_frequency) in index.iter() {
        println!("{:?} has {} unique tokens", file_path, term_frequency.len());
    }


    Ok(())
}
