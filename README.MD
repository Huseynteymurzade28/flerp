# 🌟 flerp 🌟

**Flerp Text Analysis** - Uncover content & insights from your text files! 🕵️‍♂️

Flerp is a command-line tool written in Rust that provides structural analysis, keyword extraction, and text search functionalities for your documents.

## ✨ Features

- **Structural Analysis**: Get counts of lines, words, and stanzas.
- **Keyword Extraction**: Identify the most frequent keywords in your text.
- **Text Search**: Search for specific queries within your files, with case-sensitive and case-insensitive options.
- **Cool ASCII Art Welcome**: Because why not?

## 🚀 Installation

### easy method
```
cargo install flerp
```

### if you contribute

1.  **Clone the repository (if you haven't already):**
    ```bash
    git clone https://github.com/Huseynteymurzade28/flerp.git
    cd flerp
    ```
2.  **Build and install using Cargo:**
    Ensure you have Rust and Cargo installed. If not, visit [rustup.rs](https://rustup.rs/).
    ```bash
    cargo install --path .
    ```
    This will compile `flerp` and place the executable in your Cargo binary directory (usually `~/.cargo/bin/`). Make sure this directory is in your system's `PATH`.

## 🛠️ Usage

Once installed, you can run `flerp` from your terminal:

```bash
flerp <query> <filename>
```

**Arguments:**

- `<query>`: The text string you want to search for in the file.
- `<filename>`: The path to the text file you want to analyze.

**Options:**

- **Case-sensitive search (default):**
  ```bash
  flerp "Hello" myfile.txt
  ```
- **Case-insensitive search:**
  Set the `CASE_INSENSITIVE` environment variable to any value (e.g., `true` or `1`).
  ```bash
  CASE_INSENSITIVE=true flerp "hello" myfile.txt
  ```
  Alternatively, on Windows:
  ```powershell
  $env:CASE_INSENSITIVE="true"
  flerp "hello" myfile.txt
  ```
  Or for a single command across platforms:
  ```bash
  CASE_INSENSITIVE=1 flerp "hello" myfile.txt
  ```

**Example Output:**

When you run `flerp`, you'll first see a friendly welcome message:

```
  .--.
 /  _ \
|  o o |
 \  ^ /
  `--'
   ||

    _________________________
   /                        //
  /   Flerp Text Analysis  //
 /    Content & Insights  //
/________________________//
(________________________(/
   \                      \
    \   Keywords, Stats,    \
     \   Search Results...  \
      \______________________\

```

Followed by the analysis:

```
Structural Analysis:
  Lines: 150
  Words: 2500
  Stanzas: 30

Keywords (Top 5):
  rust: 25
  code: 18
  analysis: 15
  text: 12
  flerp: 10

This is a line that contains your search query.
Another line matching the query.
...
```

## 🤝 Contributing

Contributions are welcome! If you have ideas for new features, improvements, or bug fixes, please feel free to:

1.  Fork the repository.
2.  Create a new branch (`git checkout -b feature/your-feature-name`).
3.  Make your changes.
4.  Commit your changes (`git commit -am 'Add some feature'`).
5.  Push to the branch (`git push origin feature/your-feature-name`).
6.  Create a new Pull Request.

## 📜 License

This project is licensed under the MIT License - see the `LICENSE` file for details (if you choose to add one).

---

Happy Flerping! 🎉
