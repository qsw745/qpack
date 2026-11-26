# QPack Windows 一键安装脚本 (PowerShell)
# 用法: irm https://你的域名/install.ps1 | iex
# 可通过环境变量 $env:QPACK_BASE_URL 指定服务器地址

$ErrorActionPreference = "Stop"

$BaseUrl = if ($env:QPACK_BASE_URL) { $env:QPACK_BASE_URL.TrimEnd("/") } else { "http://192.168.31.147:8080" }
$Version = if ($env:QPACK_VERSION) { $env:QPACK_VERSION } else { "0.1.2" }
$InstallerName = "QPack_${Version}_x64-setup.exe"
$DownloadUrl = "${BaseUrl}/releases/${InstallerName}"
$TempFile = Join-Path $env:TEMP $InstallerName

Write-Host ">>> 正在下载 QPack v${Version} (Windows x64)..." -ForegroundColor Cyan
Write-Host "    $DownloadUrl" -ForegroundColor DarkGray

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile -UseBasicParsing
} catch {
    Write-Error "下载失败，请检查网络连接或下载地址：$DownloadUrl"
    exit 1
}

Write-Host ">>> 正在静默安装 QPack..." -ForegroundColor Cyan
$proc = Start-Process -FilePath $TempFile -ArgumentList "/S" -Wait -PassThru

Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue

if ($proc.ExitCode -ne 0) {
    Write-Error "安装程序返回异常退出码：$($proc.ExitCode)"
    exit $proc.ExitCode
}

Write-Host ""
Write-Host "✅ QPack v${Version} 安装完成！" -ForegroundColor Green
Write-Host "   现在可以在开始菜单或桌面快捷方式中启动 QPack。" -ForegroundColor Gray
