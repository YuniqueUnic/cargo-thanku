# Cargo Thanku

[中文 README.md](./README_CN.md)

A command-line tool for generating acknowledgments for your Rust project dependencies.

## Key Features

- Generates acknowledgments in multiple formats (Markdown table/list, JSON, TOML, YAML)
- Fetches dependency information from crates.io and GitHub
- Supports concurrent processing with configurable limits
- Implements retry mechanism for failed requests
- Offers command-line completion for Bash, Zsh, Fish, PowerShell, and Elvish
- Provides internationalization support (zh/en/ja/ko/es/fr/de/it)

## Installation

Ensure you have the Rust toolchain installed on your system, then execute:

```bash
# Install cargo-thanku
cargo install cargo-thanku

# Generate shell completions (optional)
cargo thanku completions bash > ~/.local/share/bash-completion/completions/cargo-thanku
```

## Usage

### Basic Usage

```bash
# Generate acknowledgments for your project
cargo thanku

# Specify output format
cargo thanku -f markdown-table  # or markdown-list, json, toml, yaml

# Set GitHub token for more information and automatic starring
cargo thanku -t YOUR_GITHUB_TOKEN

# Change language
cargo thanku -l en  # supports zh/en/ja/ko/es/fr/de/it
```

### Advanced Options

```bash
# Configure concurrent requests
cargo thanku -j 10  # Set maximum concurrent requests to 10

# Adjust retry attempts
cargo thanku -r 5   # Set maximum retry attempts to 5

# Customize output file
cargo thanku -o custom_thanks.md

# Enable verbose logging
cargo thanku -v
```

### Command-Line Completion

Generate command-line completion scripts for various shells:

```bash
# Bash
cargo thanku completions bash > ~/.local/share/bash-completion/completions/cargo-thanku

# Zsh
cargo thanku completions zsh > ~/.zsh/_cargo-thanku

# Fish
cargo thanku completions fish > ~/.config/fish/completions/cargo-thanku.fish

# PowerShell
mkdir -p $PROFILE\..\Completions
cargo thanku completions powershell > $PROFILE\..\Completions\cargo-thanku.ps1

# Elvish
cargo thanku completions elvish > ~/.elvish/lib/cargo-thanku.elv
```

## Command-Line Arguments

| Argument            | Description                                      | Default Value     |
|---------------------|--------------------------------------------------|-------------------|
| `-i, --input`       | Input Cargo.toml file path                       | `Cargo.toml`      |
| `-o, --output`      | Output file path                                 | `thanks.md`       |
| `-f, --format`      | Output format                                    | `markdown-table`  |
| `-t, --token`       | GitHub API token                                 | -                 |
| `-l, --language`    | Language (zh/en/ja/ko/es/fr/de/it)               | `zh`              |
| `-v, --verbose`     | Enable verbose logging                           | `false`           |
| `-j, --concurrent`  | Maximum concurrent requests                      | `5`               |
| `-r, --retries`     | Maximum retry attempts                           | `3`               |

## Output Formats

### Markdown Table
```markdown
| Name | Description | Source | Stats | Status |
|------|-------------|--------|-------|--------|
| serde | Serialization framework | [GitHub](https://github.com/serde-rs/serde) | 🌟 3.5k | ✅ |
```

### Markdown List
```markdown
# Dependencies

- serde [Serialization framework](https://github.com/serde-rs/serde) (🌟 3.5k) ✅
```

### JSON/TOML/YAML
Also supports structured output formats for programmatic use.

## Important Notes

1. Setting a GitHub token (`-t` or `GITHUB_TOKEN` env) enables:
   - Fetching additional repository information
   - Automatic starring of dependency repositories
   - Higher API rate limits

2. Failed dependency processing:
   - Won't interrupt the overall process
   - Will be marked with ❌ in the output
   - Shows error messages for debugging

3. Language codes:
   - Supports flexible formats (e.g., "en", "en_US", "en_US.UTF-8")
   - Falls back to primary language code
   - Suggests similar codes for typos

## Acknowledgments

This project itself is built with many excellent Rust crates. Here are some key dependencies:

| name | description | source | stats | status |
|------|--------|--------|-------|--------|
| tracing-subscriber | Utilities for implementing and composing `tracing` subscribers.  | [GitHub](https://github.com/tokio-rs/tracing) | ❓ | ✅ |
| url | URL library for Rust, based on the WHATWG URL Standard | [GitHub](https://github.com/servo/rust-url) | ❓ | ✅ |
| toml | A native Rust encoder and decoder of TOML-formatted files and streams. Provides implementations of the standard Serialize/Deserialize traits for TOML data to facilitate deserializing and serializing Rust structures.  | [GitHub](https://github.com/toml-rs/toml) | ❓ | ✅ |
| clap | A simple to use, efficient, and full-featured Command Line Argument Parser | [GitHub](https://github.com/clap-rs/clap) | ❓ | ✅ |
| serde_yaml | YAML data format for Serde | [GitHub](https://github.com/dtolnay/serde-yaml) | ❓ | ✅ |
| cargo_metadata | structured access to the output of `cargo metadata` | [GitHub](https://github.com/oli-obk/cargo_metadata) | ❓ | ✅ |
| serde_json | A JSON serialization file format | [GitHub](https://github.com/serde-rs/json) | ❓ | ✅ |
| assert_fs | Filesystem fixtures and assertions for testing. | [GitHub](https://github.com/assert-rs/assert_fs.git) | ❓ | ✅ |
| strsim | Implementations of string similarity metrics. Includes Hamming, Levenshtein, OSA, Damerau-Levenshtein, Jaro, Jaro-Winkler, and Sørensen-Dice.  | [GitHub](https://github.com/rapidfuzz/strsim-rs) | ❓ | ✅ |
| tokio | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications.  | [GitHub](https://github.com/tokio-rs/tokio) | ❓ | ✅ |
| pretty_assertions | Overwrite `assert_eq!` and `assert_ne!` with drop-in replacements, adding colorful diffs. | [GitHub](https://github.com/rust-pretty-assertions/rust-pretty-assertions) | ❓ | ✅ |
| tokio-test | Testing utilities for Tokio- and futures-based code  | [GitHub](https://github.com/tokio-rs/tokio) | ❓ | ✅ |
| reqwest | higher level HTTP client library | [GitHub](https://github.com/seanmonstar/reqwest) | ❓ | ✅ |
| thiserror | derive(Error) | [GitHub](https://github.com/dtolnay/thiserror) | ❓ | ✅ |
| futures | An implementation of futures and streams featuring zero allocations, composability, and iterator-like interfaces.  | [GitHub](https://github.com/rust-lang/futures-rs) | ❓ | ✅ |
| rust-i18n | Rust I18n is use Rust codegen for load YAML file storage translations on compile time, and give you a t! macro for simply get translation texts. | [GitHub](https://github.com/longbridge/rust-i18n) | ❓ | ✅ |
| serde | A generic serialization/deserialization framework | [GitHub](https://github.com/serde-rs/serde) | ❓ | ✅ |
| anyhow | Flexible concrete Error type built on std::error::Error | [GitHub](https://github.com/dtolnay/anyhow) | ❓ | ✅ |
| clap_complete | Generate shell completion scripts for your clap::Command | [GitHub](https://github.com/clap-rs/clap) | ❓ | ✅ |
| tracing | Application-level tracing for Rust.  | [GitHub](https://github.com/tokio-rs/tracing) | ❓ | ✅ |

For a complete list of dependencies and their acknowledgments, run:
```bash
cargo thanku
```

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE.md) file for details. 