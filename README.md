# MP4文件合并工具

一个使用Rust和Dioxus开发的桌面应用程序，用于合并多个MP4文件，并带有实时进度条显示。

## 功能特点

- 🎬 支持选择多个MP4文件进行合并
- 📊 实时显示合并进度
- 🎨 现代化的用户界面
- 🖥️ 跨平台桌面应用（Windows、macOS、Linux）
- ⚡ 使用Rust开发，性能高效

## 环境设置

### 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 安装 VSCode 插件

- crates: Rust 包管理
- Even Better TOML: TOML 文件支持
- Better Comments: 优化注释显示
- Error Lens: 错误提示优化
- GitLens: Git 增强
- Github Copilot: 代码提示
- indent-rainbow: 缩进显示优化
- Prettier - Code formatter: 代码格式化
- REST client: REST API 调试
- rust-analyzer: Rust 语言支持
- Rust Test lens: Rust 测试支持
- Rust Test Explorer: Rust 测试概览
- TODO Highlight: TODO 高亮
- vscode-icons: 图标优化
- YAML: YAML 文件支持

### 安装 cargo generate

cargo generate 是一个用于生成项目模板的工具。它可以使用已有的 github repo 作为模版生成新的项目：

```bash
cargo install cargo-generate
```

在我们的课程中，新的项目会使用 `tyr-rust-bootcamp/template` 模版生成基本的代码：

```bash
cargo generate tyr-rust-bootcamp/template
```

### 安装 pre-commit

pre-commit 是一个代码检查工具，可以在提交代码前进行代码检查。

```bash
pipx install pre-commit
```

安装成功后运行 `pre-commit install` 即可。

### 安装 Cargo deny

Cargo deny 是一个 Cargo 插件，可以用于检查依赖的安全性。

```bash
cargo install --locked cargo-deny
```

### 安装 typos

typos 是一个拼写检查工具。

```bash
cargo install typos-cli
```

### 安装 git cliff

git cliff 是一个生成 changelog 的工具。

```bash
cargo install git-cliff
```

### 安装 cargo nextest

cargo nextest 是一个 Rust 增强测试工具。

```bash
cargo install cargo-nextest --locked
```

## 使用方法

1. 克隆或下载本项目
2. 在项目目录下运行：
   ```bash
   cargo run
   ```
3. 应用程序启动后：
   - 点击"添加文件"按钮选择要合并的MP4文件（可以选择多个）
   - 点击"选择输出文件"按钮指定合并后的输出文件路径
   - 点击"开始合并"按钮开始合并过程
   - 在合并过程中，进度条会实时显示合并进度
   - 合并完成后，会显示"合并完成!"的消息

## 技术栈

- **Rust**: 系统编程语言，提供高性能和内存安全
- **Dioxus**: 用于构建用户界面的Rust框架
- **Dioxus Desktop**: 将Dioxus应用打包为桌面应用
- **Tokio**: Rust的异步运行时
- **rfd**: 跨平台的文件对话框库

## 注意事项

- 确保所有要合并的MP4文件使用相同的编码格式
- 合并大文件时可能需要较长时间，请耐心等待
- 输出文件路径需要有足够的磁盘空间

## 许可证

MIT
