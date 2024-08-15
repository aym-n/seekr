
# Seekr

Seekr is a simple and efficient multithreaded search engine built in Rust. It enables fast and accurate searching of files based on their content by indexing directories and serving a search interface via a web server. Seekr uses the `TF-IDF` algorithm to score the relevancy of documents, ensuring that search results are both relevant and precise.

## Getting Started

### Prerequisites

- Rust (latest stable version)

### Installation

1. Clone the repository:

   ```sh
   git clone https://github.com/aym-n/seekr.git
   cd seekr
   ```

2. Build the project:

   ```sh
   cargo build --release
   ```

### Usage

Seekr can be run in two modes: `index` and `serve`.

#### Indexing Mode

To index a directory and generate a JSON file with the index:

```sh
cargo run --release index <directory>
```

- This will recursively index the specified directory and produce a `folder-name.json` file containing the index data.

#### Serve Mode

To index a directory and serve the search interface:

```sh
cargo run --release serve <directory>
```

- This will start a web server on the main thread at `127.0.0.1:8000` and also index the folder as a background process.
- The directory will be indexed on a separate thread, allowing you to search for files based on their content through the web interface.

### Example

```sh
# Index the "documents" directory
cargo run --release index documents

# Serve the "projects" directory and start the web server
cargo run --release serve projects
```
