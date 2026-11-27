# 在 Windows 环境下组装并打包 Microsoft Store 官方可上传的 .msix 包（含原生 Windows 11 一级右键菜单 COM 扩展）
param(
    [string]$Version = "",
    [string]$OutDir = "dist-windows"
)

$ErrorActionPreference = "Stop"

$RootDir = (Get-Location).Path
if (-not $Version) {
    $PkgJson = Get-Content (Join-Path $RootDir "package.json") -Raw | ConvertFrom-Json
    $Version = $PkgJson.version
}
# MSIX 版本号必须为 4 段（Major.Minor.Build.Revision）
$Version4 = if ($Version.Split(".").Count -eq 3) { "${Version}.0" } else { $Version }

$ExePath = Join-Path $RootDir "src-tauri\target\release\qpack.exe"
$DllPath = Join-Path $RootDir "src-tauri\win-shell-ext\target\release\qpack_shell_ext.dll"
$IconsDir = Join-Path $RootDir "src-tauri\icons"
$ManifestTemplate = Join-Path $RootDir "packaging\msix\AppxManifest.xml"

if (-not (Test-Path $ExePath)) { throw "未找到 $ExePath，请先执行 pnpm tauri build" }
if (-not (Test-Path $DllPath)) { throw "未找到 $DllPath，请先编译 src-tauri/win-shell-ext" }

$StageDir = Join-Path $RootDir "target-msix-layout"
if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
$AssetsDir = Join-Path $StageDir "Assets"
New-Item -ItemType Directory -Force -Path $AssetsDir | Out-Null

# 1. 复制主程序与 Windows 11 一级右键菜单 COM DLL
Copy-Item $ExePath (Join-Path $StageDir "qpack.exe")
Copy-Item $DllPath (Join-Path $StageDir "qpack_shell_ext.dll")

# 2. 复制微软商店规范图标资源
$LogoFiles = @(
    "StoreLogo.png",
    "Square44x44Logo.png",
    "Square71x71Logo.png",
    "Square150x150Logo.png",
    "Square310x310Logo.png"
)
foreach ($f in $LogoFiles) {
    $src = Join-Path $IconsDir $f
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $AssetsDir $f)
    } else {
        Copy-Item (Join-Path $IconsDir "128x128.png") (Join-Path $AssetsDir $f)
    }
}

# 3. 渲染带 4 段版本号的 AppxManifest.xml
$ManifestContent = (Get-Content $ManifestTemplate -Raw -Encoding UTF8).Replace("__VERSION_4__", $Version4)
[System.IO.File]::WriteAllText((Join-Path $StageDir "AppxManifest.xml"), $ManifestContent, [System.Text.UTF8Encoding]::new($false))

# 4. 查找 Windows SDK 自带的 makeappx.exe 并打包 .msix
$MakeAppx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1

if (-not $MakeAppx) {
    throw "未找到 Windows SDK makeappx.exe"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutMsix = Join-Path (Resolve-Path $OutDir).Path "QPack_${Version}_x64.msix"

Write-Host ">>> 使用 $($MakeAppx.FullName) 打包 MSIX: $OutMsix" -ForegroundColor Cyan
& $MakeAppx.FullName pack /d $StageDir /p $OutMsix /o
if ($LASTEXITCODE -ne 0) {
    throw "makeappx 打包失败，退出码: $LASTEXITCODE"
}

Write-Host "✅ 已生成 Microsoft Store 可上传安装包: $OutMsix" -ForegroundColor Green
