# Cargo Thanku

[English README.md](./README.md)

一个用于生成 Rust 项目依赖致谢的命令行工具。

## 主要特性

- 支持多种输出格式（Markdown 表格/列表、JSON、TOML、YAML）
- 自动从 crates.io 和 GitHub 获取依赖信息
- 支持可配置的并发处理
- 实现请求失败重试机制
- 提供命令行自动补全（支持 Bash、Zsh、Fish、PowerShell 和 Elvish）
- 支持多语言（中文/英文/日文/韩文/西班牙文/法文/德文/意大利文）

## 安装

确保系统已安装 Rust 工具链，然后执行：

```bash
# 安装 cargo-thanku
cargo install cargo-thanku

# 生成 shell 补全脚本（可选）
cargo thanku completions bash > ~/.local/share/bash-completion/completions/cargo-thanku
```

## 使用方法

### 基本用法

```bash
# 为你的项目生成致谢文档
cargo thanku

# 指定输出格式
cargo thanku -f markdown-table  # 可选：markdown-list, json, toml, yaml

# 设置 GitHub 令牌以获取更多信息并自动点赞
cargo thanku -t YOUR_GITHUB_TOKEN

# 切换语言
cargo thanku -l zh  # 支持 zh/en/ja/ko/es/fr/de/it
```

### 高级选项

```bash
# 配置并发请求数
cargo thanku -j 10  # 设置最大并发请求数为 10

# 调整重试次数
cargo thanku -r 5   # 设置最大重试次数为 5

# 自定义输出文件
cargo thanku -o custom_thanks.md

# 启用详细日志
cargo thanku -v

# 过滤掉相对路径导入的 libs
cargo thanku --no-relative-libs
```

### 命令行补全

为不同的 shell 生成命令行补全脚本：

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

## 命令行参数

| 参数                | 描述                                               | 默认值          |
|---------------------|----------------------------------------------------|-----------------|
| `-i, --input`       | 输入的 Cargo.toml 文件路径                         | `Cargo.toml`    |
| `-o, --output`      | 输出文件路径                                       | `thanks.md`     |
| `-f, --format`      | 输出格式                                           | `markdown-table`|
| `-t, --token`       | GitHub API 令牌                                    | -               |
| `-l, --language`    | 语言 (zh/en/ja/ko/es/fr/de/it)                     | `zh`            |
| `-v, --verbose`     | 启用详细日志                                       | `false`         |
| `-j, --concurrent`  | 最大并发请求数                                     | `5`             |
| `-r, --retries`     | 最大重试次数                                       | `3`             |
| `--no-relative-libs`| Filter out libraries imported with relative paths  | `false`         |

## 输出格式

### Markdown 表格

```markdown
| 名称 | 描述 | 来源 | 统计 | 状态 |
|------|------|------|------|------|
| [serde](https://crates.io/crates/serde) | 序列化框架 | [GitHub](https://github.com/serde-rs/serde) | 🌟 3.5k | ✅ |
```

### Markdown 列表

```markdown
# 依赖项

- [serde](https://crates.io/crates/serde) [序列化框架](https://github.com/serde-rs/serde) (🌟 3.5k) ✅
```

### JSON/TOML/YAML
同时支持结构化输出格式，方便程序化使用。

## 重要说明

1. 设置 GitHub 令牌（通过 `-t` 或环境变量 `GITHUB_TOKEN`）可以：
   - 获取更多仓库信息
   - 自动获取依赖仓库 stars
   - 提高 API 访问限制

2. 依赖处理失败时：
   - 不会中断整体处理过程
   - 在输出中会标记为 ❌
   - 显示错误信息以便调试

3. 语言代码支持：
   - 支持灵活的格式（如 "zh"、"zh_CN"、"zh_CN.UTF-8"）
   - 自动提取主要语言代码
   - 对于拼写错误会提供相似代码建议

## 致谢

本项目本身也使用了许多优秀的 Rust crate。以下是一些主要依赖：

> [!TIP]
> 由 `cargo-thanku` 工具生成

| 名称 | 描述 | 来源 | 统计 | 状态 |
|------|--------|--------|-------|--------|
|🔍|Normal| | | |
| [anyhow](https://crates.io/crates/anyhow) | Flexible concrete Error type built on std::error::Error | [GitHub](https://github.com/dtolnay/anyhow) | ❓ | ✅ |
| [cargo_metadata](https://crates.io/crates/cargo_metadata) | structured access to the output of `cargo metadata` | [GitHub](https://github.com/oli-obk/cargo_metadata) | ❓ | ✅ |
| [clap](https://crates.io/crates/clap) | A simple to use, efficient, and full-featured Command Line Argument Parser | [GitHub](https://github.com/clap-rs/clap) | ❓ | ✅ |
| [clap_complete](https://crates.io/crates/clap_complete) | Generate shell completion scripts for your clap::Command | [GitHub](https://github.com/clap-rs/clap) | ❓ | ✅ |
| [futures](https://crates.io/crates/futures) | An implementation of futures and streams featuring zero allocations, composability, and iterator-like interfaces.  | [GitHub](https://github.com/rust-lang/futures-rs) | ❓ | ✅ |
| [reqwest](https://crates.io/crates/reqwest) | higher level HTTP client library | [GitHub](https://github.com/seanmonstar/reqwest) | ❓ | ✅ |
| [rust-i18n](https://crates.io/crates/rust-i18n) | Rust I18n is use Rust codegen for load YAML file storage translations on compile time, and give you a t! macro for simply get translation texts. | [GitHub](https://github.com/longbridge/rust-i18n) | ❓ | ✅ |
| [serde](https://crates.io/crates/serde) | A generic serialization/deserialization framework | [GitHub](https://github.com/serde-rs/serde) | ❓ | ✅ |
| [serde_json](https://crates.io/crates/serde_json) | A JSON serialization file format | [GitHub](https://github.com/serde-rs/json) | ❓ | ✅ |
| [serde_yaml](https://crates.io/crates/serde_yaml) | YAML data format for Serde | [GitHub](https://github.com/dtolnay/serde-yaml) | ❓ | ✅ |
| [strsim](https://crates.io/crates/strsim) | Implementations of string similarity metrics. Includes Hamming, Levenshtein, OSA, Damerau-Levenshtein, Jaro, Jaro-Winkler, and Sørensen-Dice.  | [GitHub](https://github.com/rapidfuzz/strsim-rs) | ❓ | ✅ |
| [thiserror](https://crates.io/crates/thiserror) | derive(Error) | [GitHub](https://github.com/dtolnay/thiserror) | ❓ | ✅ |
| [tokio](https://crates.io/crates/tokio) | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications.  | [GitHub](https://github.com/tokio-rs/tokio) | ❓ | ✅ |
| [toml](https://crates.io/crates/toml) | A native Rust encoder and decoder of TOML-formatted files and streams. Provides implementations of the standard Serialize/Deserialize traits for TOML data to facilitate deserializing and serializing Rust structures.  | [GitHub](https://github.com/toml-rs/toml) | ❓ | ✅ |
| [tracing](https://crates.io/crates/tracing) | Application-level tracing for Rust.  | [GitHub](https://github.com/tokio-rs/tracing) | ❓ | ✅ |
| [tracing-subscriber](https://crates.io/crates/tracing-subscriber) | Utilities for implementing and composing `tracing` subscribers.  | [GitHub](https://github.com/tokio-rs/tracing) | ❓ | ✅ |
| [url](https://crates.io/crates/url) | URL library for Rust, based on the WHATWG URL Standard | [GitHub](https://github.com/servo/rust-url) | ❓ | ✅ |
|🔧|Development| | | |
| [assert_fs](https://crates.io/crates/assert_fs) | Filesystem fixtures and assertions for testing. | [GitHub](https://github.com/assert-rs/assert_fs.git) | ❓ | ✅ |
| [pretty_assertions](https://crates.io/crates/pretty_assertions) | Overwrite `assert_eq!` and `assert_ne!` with drop-in replacements, adding colorful diffs. | [GitHub](https://github.com/rust-pretty-assertions/rust-pretty-assertions) | ❓ | ✅ |
| [tokio-test](https://crates.io/crates/tokio-test) | Testing utilities for Tokio- and futures-based code  | [GitHub](https://github.com/tokio-rs/tokio) | ❓ | ✅ |

要查看完整的依赖列表和致谢，请运行：
```bash
cargo thanku
```

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](./LICENSE.md) 文件。 