<h1 align="center"><img src="web/asserts/ncmconvertor_logo.png" alt="ncmconvertor — 浏览器本地 NCM 转换工具" width="720"></h1>

[在线使用](https://yazhuoliu.com/ncmconverter/) · [源代码](https://github.com/Yazhuo-Liu/ncmconverter) · [English](README.md) · **简体中文**

[![测试](https://github.com/Yazhuo-Liu/ncmconverter/actions/workflows/test.yml/badge.svg)](https://github.com/Yazhuo-Liu/ncmconverter/actions/workflows/test.yml)

`ncmconvertor` 可以将网易云音乐的 NCM 缓存文件还原为原始音频。本 Fork 保留现有 Rust CLI 和核心库，同时提供完全在用户设备上运行的浏览器版本。

> [!IMPORTANT]
> 文件内容只在当前浏览器标签页中处理，绝不会上传到服务器。

## Web 版

Web 版支持批量选择或拖放 `.ncm` 文件，在模块类型的 Web Worker 中调用现有 Rust/WASM 解密与元数据逻辑，根据解密后的音频数据识别实际格式，并输出 MP3、FLAC、M4A 或 OGG。

每个文件独立显示状态。转换成功后可以使用正确的扩展名和 MIME 类型进行试听或下载。界面支持中文和英文切换。

- 在线版本：[yazhuoliu.com/ncmconverter](https://yazhuoliu.com/ncmconverter/)
- 源码：[Yazhuo-Liu/ncmconverter](https://github.com/Yazhuo-Liu/ncmconverter)

页面不包含文件上传接口、分析、广告、统计 SDK、远程字体或运行时 CDN 依赖，部署产物全部为静态文件。

### 环境要求

- Node.js 20.19 或更高版本
- `rust-toolchain.toml` 指定的 Rust 工具链
- Rust `wasm32-unknown-unknown` 目标
- wasm-pack 0.13.1

如果尚未安装 Rust 目标和 wasm-pack，请执行：

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.13.1 --locked
```

### 本地开发

```bash
cd web
npm install
npm run dev
```

`npm run dev` 会先构建浏览器 WASM 模块，然后启动 Vite 开发服务器。

### 生产构建

```bash
cd web
npm install
npm run build
```

构建命令会先将 Rust 编译为浏览器 WASM，再构建前端。最终静态文件位于 `web/dist/`。所有资源均使用相对 URL，因此可以部署在 GitHub Pages 的 `/ncmconverter/` 子路径下。

使用仓库中的全部 NCM 测试文件验证浏览器 WASM 转换：

```bash
npm run verify:wasm
```

### GitHub Pages 部署

[`.github/workflows/pages.yml`](.github/workflows/pages.yml) 会在默认分支更新时自动构建并部署 Web 版。工作流安装固定版本的 Rust 和 wasm-pack、安装 Node 依赖、构建项目、上传 `web/dist/`，最后通过 GitHub 官方 Pages Actions 完成部署。

首次部署前，请前往 GitHub 仓库的 **Settings → Pages → Build and deployment**，将 **Source** 设置为 **GitHub Actions**。

### 浏览器内存限制

浏览器转换时可能需要同时容纳加密输入、WASM 线性内存、解密结果，以及用于试听和下载的 Blob，因此峰值内存占用可能达到文件大小的数倍。

文件会串行转换。页面会对超过 100 MiB 的文件显示提醒，但不会设置无理由的大小限制。实际可处理上限取决于浏览器、操作系统和设备可用内存。在移动设备或内存较小的设备上，建议减少一次选择的文件数量。

## 命令行版本

项目名称为 `ncmconvertor`。为了保持命令行兼容性，现有可执行文件和 Rust crate 仍然使用 `ncmc` 名称。

从当前仓库源码安装 CLI：

```bash
cargo install --path crates/ncmc
```

也可以在仓库根目录直接运行：

```bash
cargo run -p ncmc -- path/to/your/file.ncm
```

如果尚未安装 Rust，请参考 [rustup](https://rustup.rs/)。

### CLI 使用方法

```bash
# 转换单个文件
ncmc path/to/your/file.ncm

# 只扫描目录顶层的 .ncm 和 .NCM 文件
ncmc path/to/music-directory

# 递归扫描一个或多个目录，也可以混合传入文件
ncmc --recursive path/to/music-directory another/file.ncm

# 不覆盖已经存在的音频文件
ncmc --recursive --skip-existing path/to/music-directory

# 导出解密后的密钥、元数据、封面和音频数据
ncmc --dump path/to/your/file.ncm
```

目录扫描只选择 NCM 文件，扩展名使用 ASCII 大小写不敏感匹配，并且不会跟随扫描过程中发现的符号链接。直接指定的文件不受扩展名过滤。

转换任务串行执行。单个文件失败不会中断其他文件；每次运行结束时会显示 `total`、`succeeded`、`skipped` 和 `failed` 汇总。文件发现或转换失败时，命令会以非零状态码退出。

## 致谢与许可

`ncmconvertor` 是 [ghostroller/ncmc](https://github.com/ghostroller/ncmc) 的 Fork，并保留其 Rust CLI 与库能力。

原项目致谢：[anonymous5l/ncmdump](https://github.com/anonymous5l/ncmdump)。

本项目基于 [MIT License](LICENSE) 发布；上游项目和贡献者署名可在原仓库历史中查阅。
