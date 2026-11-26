# QPack

极简、高性能、小体积的跨平台（macOS & Windows）解压缩软件，基于 **Tauri 2 + Rust + React + TypeScript + Tailwind CSS** 构建。

## 核心特性（Phase 1 MVP）

- **拖拽即用**：拖入压缩包立即打开浏览与解压；拖入文件/文件夹一键打包压缩。
- **智能解压**：自动分析包内目录结构，避免顶层散落多个文件弄乱目录（告别“解压炸弹”），单目录则不额外套娃。
- **跨平台中文防乱码**：内置 `chardetng` + `encoding_rs` 自动识别 GBK / Big5 / Shift-JIS / UTF-8，支持工具栏一键切换编码重新加载。
- **双引擎架构**：
  - **Rust 原生引擎**：原生高速处理 `ZIP`、`7Z`、`TAR`、`TAR.GZ`、`TAR.BZ2`、`TAR.XZ`、`TAR.ZST`、`GZ`、`BZ2`、`XZ`、`ZST`，支持 ZIP AES-256 加密与自动过滤 `.DS_Store` / `__MACOSX` / `Thumbs.db`。
  - **7zz 扩展引擎**：自动探测内置 `binaries/7zz` 或系统 `7zz`/`7z`，即刻解锁 `RAR / RAR5`、`ISO`、`DMG`、`CAB` 等格式。
- **高性能虚拟滚动**：采用 `@tanstack/react-virtual`，浏览数万文件列表依然流畅。
- **安全防护**：内置 Zip Slip 路径穿越拦截，支持部分提取与双击快速预览。

## 本地开发

```bash
# 安装前端依赖
pnpm install

# 启动桌面端开发模式
pnpm tauri dev
```

## 测试与构建

```bash
# 运行 Rust 核心单元测试（智能解压、编码防乱码、Zip Slip、多格式压缩/解压闭环）
cargo test --manifest-path src-tauri/Cargo.toml

# 打包当前平台安装包（macOS .dmg / Windows .msi）
pnpm tauri build
```

## 可选：启用 RAR / ISO / DMG 全格式支持

- **macOS**：`brew install sevenzip`
- **Windows**：安装 [7-Zip](https://www.7-zip.org/) 并确保 `7z.exe` 或 `7zz.exe` 在系统 `PATH` 中，或直接将 `7zz` 可执行文件放入应用同级 `binaries/` 目录。

## 代码签名与开源协议（Code Signing Policy）

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

本项目以 [MIT License](./LICENSE) 开源，Windows 正式发布安装包通过 GitHub Actions（[release.yml](./.github/workflows/release.yml)）自动构建，并由 **SignPath Foundation** 提供免费开源代码签名。详细角色与安全说明见 [CODE_SIGNING.md](./CODE_SIGNING.md)。
