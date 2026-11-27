# QPack Microsoft Store 提审资料清单（直接复制粘贴）

---

## 1. 基本属性（Properties）
- **类别（Category）**：`效率`（Productivity） → 子类别：`实用工具和工具`（Utilities & tools）
- **定价和可用性（Pricing and availability）**：免费（Free），所有市场可见
- **年龄分级（Age ratings）**：按 IARC 问卷全选「否」（无暴力/赌博/社交/内购/用户生成内容），生成 **3+ / Everyone** 分级
- **隐私政策 URL（Privacy policy URL）**：`https://github.com/qsw745/qpack/blob/main/CODE_SIGNING.md`
- **支持联系人 / 官网 URL**：`https://github.com/qsw745/qpack`

---

## 2. 简体中文商品页信息（Chinese - Simplified）

### 简短描述（Short description）
```text
极简、高性能、无广告的跨平台解压缩工具。支持智能防套娃解压、跨平台中文防乱码与 Windows 11 原生右键菜单。
```

### 详细描述（Description）
```text
QPack 是一款基于 Rust 原生引擎与 Tauri 2 构建的轻量级桌面解压缩工具，告别臃肿广告与繁杂弹窗，专注纯净、高速、可靠的文件压缩与解压体验。

【核心亮点】
• 智能防套娃解压：解压前自动分析压缩包顶层目录结构。顶层散落多个文件时自动新建同名目录防止弄乱桌面；已有单根目录时直接释放，告别“文件夹套文件夹”。
• 跨平台中文防乱码：内置自动编码检测引擎，完美识别 GBK、Big5、Shift-JIS 与 UTF-8 文件名，并支持工具栏一键切换编码即时重载。
• Windows 11 原生现代一级右键菜单：右键文件或文件夹直接一步压缩为 ZIP，右键压缩包一步智能解压，无需再点“显示更多选项”。
• 冲突处理与进度可视化：遇到同名目录/文件时提供“自动新建目录 / 覆盖 / 跳过”清晰确认；居中实时进度条与解压后自动打开目标文件夹，状态一目了然。
• 双引擎全格式覆盖：Rust 原生高速引擎开箱即用支持 ZIP（含 AES-256 加密）、7Z、TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST、GZ、BZ2、XZ、ZST；自动探测系统 7z/7zz 即可解锁 RAR/RAR5、ISO、CAB、DMG 等格式。
• 安全纯净：内置 Zip Slip 路径穿越安全防护，压缩与解压自动过滤 .DS_Store、__MACOSX 与 Thumbs.db 等冗余系统文件。
```

### 应用功能点（App features，最多 20 条，逐条填入）
1. 智能防套娃解压，自动识别顶层目录结构
2. 跨平台中文防乱码（GBK / Big5 / Shift-JIS / UTF-8 自动检测与切换）
3. Windows 11 原生现代一级右键菜单（压缩与智能解压一步直达）
4. 解压同名冲突智能确认（自动新建目录 / 覆盖 / 跳过）与实时进度条
5. Rust 原生高速引擎，支持 ZIP AES-256 加密、7Z、TAR、ZST 等主流格式
6. 支持虚拟滚动流畅浏览数万文件、搜索过滤与双击快速预览
7. 纯本地运行、零广告弹窗、自动过滤跨平台垃圾文件

### 搜索词（Search terms，最多 7 个）
`解压`, `压缩`, `zip`, `7z`, `rar`, `解压缩`, `归档`

### 此版本中的新增功能（What's new in this version）
```text
• 首发上架 Microsoft Store，内置 Windows 11 原生现代一级右键菜单
• 支持双击压缩包窗口预览、自定义应用内右键菜单与居中实时解压进度条
• 新增解压同名文件/文件夹冲突确认（自动新建目录 / 覆盖 / 跳过）及解压完成自动打开目标目录
```

---

## 3. 英文商品页信息（English，可选添加）
- **Short description**：`Minimalist, ultra-fast archive & compression utility built with Rust. Smart anti-nesting extraction, CJK encoding fix, and native Windows 11 context menu.`
- **Description**：`QPack is a lightweight, ad-free archive manager powered by Rust and Tauri 2. It features smart extraction (prevents archive bombs and nested folders), automatic CJK/GBK/UTF-8 filename decoding, native Windows 11 Explorer context menus, conflict resolution prompts, real-time extraction progress, and Zip Slip protection.`

---

## 4. 审核备注（Notes for certification — 强烈建议填写，可大幅加速通过）
```text
QPack is a local desktop archive and compression utility built with Rust and Tauri 2 (MIT open-source: https://github.com/qsw745/qpack).
- The `runFullTrust` capability is required to read/write user-selected local files and archives, and to register the native Windows 11 File Explorer context menu COM extension (`qpack_shell_ext.dll`).
- No login, account, or remote server is required to test the app. Simply drag and drop any .zip/.7z/.tar archive into the main window to preview or extract, or drag files/folders to compress.
- QPack does not collect or transmit any user data.
```

---

## 5. 商店视觉素材（均已生成在本目录）
- **300×300 应用徽标**：`packaging/store-listing/store-icon-300x300.png`
- **桌面端截图 1（1920×1080）**：`packaging/store-listing/screenshot-1-home.png`
- **桌面端截图 2（1920×1080）**：`packaging/store-listing/screenshot-2-browser.png`
