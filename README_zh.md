<!-- markdownlint-disable-file MD033 --><!-- we are OK with inline HTML since we use <kbd> tags -->

[English](README.md) | 中文

# Zed Settings Sync

**Zed Settings Sync** 是一个 [Zed](https://zed.dev) 编辑器扩展，通过 LSP 自动将用户级配置文件同步到远程存储。支持的同步源：**GitHub Gist** 和 **WebDAV**。

ℹ️ 该扩展不会同步项目级配置文件，因为将它们直接纳入项目版本控制更为实用。

使用 LSP 是一种变通方案，受限于当前 Zed 扩展 API 的能力。

_该方案深受 [Zed Discord Presence](https://github.com/xhyrom/zed-discord-presence) 扩展的启发。_

## 安装

由于对应的 [Zed 扩展仓库 PR](https://github.com/zed-industries/extensions/pull/4557) 被[拒绝](https://github.com/zed-industries/extensions/pull/4557#issuecomment-3834080836)，目前只能通过[开发模式安装](#开发环境安装)。这不是最理想或最便捷的方式，但在 Zed 原生设置同步功能到来之前，这是目前唯一可用的方案。

## 配置

### 同步源

扩展支持两种同步源：**GitHub Gist**（默认）和 **WebDAV**。

#### GitHub Gist

##### 如果你已有 Zed 但还没有 Gist

1. 创建一个具有 `gist` 权限范围的 GitHub token（[详细指南](docs/CREATE_GITHUB_TOKEN.md)）
2. 准备一个 Gist（[详细指南](docs/CREATE_SETTINGS_GIST.md)）
3. 在 Zed 设置文件中添加凭证：

```jsonc
{
  "lsp": {
    "settings-sync": {
      "initialization_options": {
        "github_token": "gho_my-shiny-token",
        "gist_id": "deadbeefdeadbeefdeadbeefdeadbeef"
      }
    }
  }
}
```

#### WebDAV

在 Zed 设置文件中添加以下内容：

```jsonc
{
  "lsp": {
    "settings-sync": {
      "initialization_options": {
        "sync_source": "webdav",
        "webdav_url": "https://your-webdav-server.example.com",
        "webdav_username": "your-username",
        "webdav_password": "your-password",
        "webdav_remote_path": "/zed-settings-sync"  // 可选，默认为 "/zed-settings-sync"
      }
    }
  }
}
```

扩展使用 Basic Auth 认证，通过 `PUT` 请求将文件存储到 `{webdav_url}{webdav_remote_path}/{filename}`。

### 如果你刚安装了 Zed，想从已有的同步源拉取配置

⚠️ 由于当前 Zed 扩展功能的限制，扩展本身无法从 GitHub Gist 或 WebDAV 服务器加载配置。为此提供了 CLI 工具。

#### 从 GitHub Gist 加载

确保你已准备好 [GitHub token](docs/CREATE_GITHUB_TOKEN.md) 和 [Gist ID](docs/CREATE_SETTINGS_GIST.md)。

1. 安装 [eget](https://github.com/zyedidia/eget)
2. 运行 `eget vittorius/zed-settings-sync --to=~/.local/bin`（或其他你偏好的目标目录）
3. 在 eget 提供的选项中选择 `zed-settings-sync-cli` 二进制文件
4. 运行 `zed-settings-sync-cli load` 并按提示操作，选择 `github` 作为同步源

（当然，你也可以从 [GitHub Releases](https://github.com/vittorius/zed-settings-sync/releases) 手动下载并解压二进制文件）

#### 从 WebDAV 加载

1. 下载 `zed-settings-sync-cli` 二进制文件（见上文）
2. 运行 `zed-settings-sync-cli load`，选择 `webdav` 作为同步源，然后输入 WebDAV URL、用户名、密码和远程路径

## 使用

### 同步配置文件

配置完成后，你可以：

- 编辑设置文件（或 <kbd>zed: open settings file</kbd>）
- 编辑快捷键文件（或 <kbd>zed: open keymap file</kbd>）
- 编辑任务（<kbd>zed: open tasks</kbd>）
- 编辑调试任务（<kbd>zed: open debug tasks</kbd>）

文件保存后（无论是手动保存还是通过自动保存），都会同步到配置的同步源（GitHub Gist 或 WebDAV）。

ℹ️ Zed 某次更新后添加了图形化界面来编辑设置和快捷键。
默认情况下，运行 <kbd>zed: open settings</kbd> 或 <kbd>zed: open keymap</kbd> 时会弹出该界面。
使用图形编辑器时，请点击 `Edit in settings.json` 或 `Edit in keymap.json`。
之后可以继续使用图形编辑器，**只需保持对应的 JSON 配置文件处于打开状态**，
以便 LSP 能够捕获并同步更改。当然，你也可以像以前一样手动编辑配置文件。

另一种方式是交换快捷键映射，让 <kbd>zed: open settings</kbd> 直接打开设置文件（快捷键文件同理）：

```json
{
  "bindings": {
    "cmd-,": "zed::OpenSettingsFile",
    "alt-cmd-,": "zed::OpenSettings"
  }
}
```

## 故障排查

- 打开 LSP 日志（<kbd>dev: open language server logs</kbd>），找到对应设置文件的 `settings-sync` LSP 服务器实例，检查其日志
- 在 GitHub 上提交 [issue](https://github.com/vittorius/zed-settings-sync/issues/new)

## 开发

### 开发环境搭建

前置要求：

- [Git](https://git-scm.com/)
- [Rust](https://rust-lang.org)，推荐使用 [rustup](https://rustup.rs) 安装
- [Nextest](https://nexte.st/) 测试运行器（[部分测试](common/src/config.rs)依赖它来避免跨线程同步）
- [iprecommit](https://github.com/iafisher/iprecommit) 用于 Git hooks
  - 安装 `uv`
  - 进入克隆的仓库目录
  - 执行 `uv venv`
  - 执行 `uvx pip install iprecommit`
  - 执行 `uvx precommit install`

### 开发环境安装

1. 克隆仓库
2. <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd>，选择 <kbd>zed: install dev extension</kbd>
   1. 你可能需要安装额外的构建工具依赖，如 macOS 的 XCode、Windows 的 Visual Studio 构建工具等。安装扩展时如果看到 `Error: Failed to install dev extension: failed to compile Rust extension` 弹窗，请查看 <kbd>zed: open log</kbd> 了解详情。
3. 选择克隆的仓库目录
4. 安装扩展后，重新加载工作区（<kbd>workspace: reload</kbd>）以启动 LSP 服务器

### LSP 服务器快速迭代

运行

```shell
cargo xtask-lsp-install
```

将 LSP 服务器二进制文件从本地仓库安装到 Zed 扩展目录。
然后在 Zed 中执行 <kbd>workspace: reload</kbd>，让开发扩展加载更新后的 LSP 服务器二进制文件。
