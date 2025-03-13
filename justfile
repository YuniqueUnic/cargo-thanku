# 根据操作系统自动设置 shell
set windows-shell := ["pwsh", "-c"]
set shell := ["bash", "-c"]

# 默认显示帮助信息
default:
    @just --list

# 安装依赖工具
setup:
    cargo install cargo-edit cargo-watch cargo-release
    # Already installed that those commands can be run directly
    # so, no need to install just
    # cargo install just

# ai 生成 git commit message 并且 commit. 需要先登录 github
git-commit:
    git add . && git commit -m "$(git status | aichat -r git)"

# ai 生成 git commit message 并且 push. 需要先登录 github
git-push:
    git add . && git commit -m "$(git status | aichat -r git)" && git push

# 检查代码格式
fmt:
    cargo fmt --all -- --check

# 运行 clippy 检查
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
test:
    cargo test --all-features

# 构建发布版本
build:
    cargo build --release

# 运行所有检查（格式、lint、测试）
check: fmt lint test

# 清理构建产物
clean:
    cargo clean

# 启动开发模式（代码变更自动重新编译）
dev:
    cargo watch -x run

# 创建新的发布版本
# 用法: just release [major|minor|patch] [--execute/留空也是--dry-run]
release level x="": git-commit
    cargo release {{level}} --no-publish --no-verify {{x}} -v

release-only level x="":
    cargo release {{level}} --no-publish --no-verify {{x}} -v


# 发布到 crates.io
publish:
    cargo publish

# 生成 CHANGELOG
changelog:
    git cliff -o CHANGELOG.md

# 安装到本地系统
install:
    cargo install --path .

# 运行带有调试日志的程序
run-debug *args:
    RUST_LOG=debug cargo run -- {{args}}

# 运行带有跟踪日志的程序
run-trace *args:
    RUST_LOG=trace cargo run -- {{args}}