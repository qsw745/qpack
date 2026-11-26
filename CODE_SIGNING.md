# 代码签名政策（Code Signing Policy）

本程序（QPack）的 Windows 正式发布安装包通过 **[SignPath.io](https://signpath.io)** 提供的免费开源代码签名服务进行签名，证书由 **[SignPath Foundation](https://signpath.org)** 颁发。

> Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

## 成员与权限角色（Team Roles）
- **Authors / Committers（代码提交者）**：QPack 仓库维护者及公开 Pull Request 贡献者（所有代码变更公开透明可审计）。
- **Reviewers / Approvers（签名审核者）**：仓库核心维护者，负责在 SignPath 后台审核并批准正式发布版本的签名请求。

## 构建与签名流程（Build & Signing Process）
1. 所有参与签名的 Windows 安装包均仅通过公开的 GitHub Actions 工作流（`.github/workflows/release.yml`）从源码自动构建产出，不接受开发者本地手工上传的二进制文件。
2. 构建产物仅在推送正式版本标签（`v*`）或仓库维护者手动触发发布流程时提交至 SignPath 进行 Authenticode 代码签名。
3. 签名完成后的安装包直接发布至 GitHub Releases 与官网下载中心，并附带 Tauri Updater Ed25519 签名清单 `latest.json`。

## 隐私声明（Privacy Policy）
QPack 为纯本地运行的桌面解压缩工具，不收集、不上传任何用户文件数据或个人隐私信息。应用内自动更新功能仅向配置的公开更新清单地址发起版本检查请求。
