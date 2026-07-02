# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Zed 编辑器的扩展，通过 LSP 将用户配置文件（settings.json、keymap.json 等）同步到 GitHub Gist。LSP 方式是 Zed 扩展 API 限制的变通方案。

## 常用命令

```bash
# 构建
cargo build

# 测试（必须使用 nextest，不能用 cargo test）
cargo nextest run

# Lint
cargo clippy --workspace -- -D warnings

# 格式化
cargo fmt

# 快速 LSP 迭代：构建并安装 LSP 二进制到 Zed 扩展目录
cargo xtask-lsp-install
```

**重要**：测试必须用 `cargo nextest run`，不能用 `cargo test`。部分测试（`common/src/config.rs`）依赖 nextest 的进程隔离，不用跨线程同步。若未安装 nextest 会直接 `exit(1)`。

## 工具链

- Rust stable 1.95+（`rust-toolchain.toml`）
- Edition 2024
- 格式化：`rustfmt.toml`，`reorder_imports = true`

## 工作区结构

```
Cargo.toml          # workspace root + Zed 扩展入口（cdylib）
├── common/         # 共享库：Config、InteractiveIO trait、GithubClient（octocrab）、sync 逻辑
├── lsp/            # LSP 服务器：tower-lsp，文件监听 → GitHub 同步
├── cli/            # CLI 工具：从 Gist 拉取配置到本地（clap）
├── test_support/   # 测试辅助：临时目录、mock 路径、fake token、nextest_only! 宏
├── src/lib.rs      # Zed 扩展入口（cdylib，仅依赖 zed_extension_api）
└── xtask-lsp-install/  # 将 LSP binary 复制到 Zed 扩展目录
```

## 核心架构

### LSP 服务器（`lsp/`）

- `Backend` 实现 `tower_lsp::LanguageServer`，处理 `initialize`/`did_open`/`did_close`
- `AppState` 持有 `PathStore`（文件监听器）和 `GithubClient`
- `did_open` 时，判断是否为 Zed 配置文件（`ZedConfigFilePath`），是则加入监听
- `did_close` 时移除监听
- 文件变更由 `PathWatcher`（`notify` crate）触发，自动同步到 GitHub Gist
- 配置通过 LSP `initialization_options` 传入（`github_token` + `gist_id`）

### 共享库（`common/`）

- `Config`：从 Zed settings 文件或交互式输入读取配置
- `GithubClient` trait + `octocrab` 实现：与 GitHub Gist API 交互
- `InteractiveIO` trait：抽象终端 I/O，便于测试 mock

### CLI（`cli/`）

- `zed-settings-sync-cli load [--force]`：从 Gist 拉取配置到本地
- 交互式输入 GitHub token（隐藏输入）和 Gist ID

## Mock 测试模式

项目广泛使用 `mockall`，通过 `mockall_double::double` 实现编译期 mock 替换：

- `#[cfg_attr(feature = "test-support", mockall::automock)]` 在 trait impl 上生成 mock
- `#[double] use Xxx;` 在测试时自动替换为 `MockXxx`
- `test_support` crate 提供临时目录、mock 路径、fake token
- `nextest_only!()` 宏通过 `#[ctor]` 确保只在 nextest 环境运行

## Pre-commit 钩子

使用 [iprecommit](https://github.com/iafisher/iprecommit)（需 `uv` 安装）：
- `NoForbiddenStrings`：检查禁止的字符串
- `RustFmt`：格式化检查（`.rs` 文件）
- `CargoClippy`：workspace clippy，warnings 视为错误

## 关键依赖

- `tower-lsp`：LSP 服务器框架
- `octocrab`：GitHub API 客户端
- `notify`：文件系统监听
- `paths`（来自 zed-industries/zed.git）：Zed 配置目录路径
- `mockall` + `mockall_double`：mock 框架
- `assert_fs`：测试用临时文件系统
