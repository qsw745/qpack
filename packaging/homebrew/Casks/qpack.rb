cask "qpack" do
  version "0.1.2"
  sha256 "9ad13f84358cd74d59ef53eb006ded756c7d636f576b796a94c20ebbd40a37e0"

  # 上线正式服务器后，将下方域名替换为你的正式 HTTPS 域名
  url "http://192.168.31.147:8080/releases/QPack_#{version}_aarch64.dmg"
  name "QPack"
  desc "极简、高性能、跨平台解压缩工具（智能防套娃、中文防乱码、双引擎）"
  homepage "http://192.168.31.147:8080"

  auto_updates true
  depends_on macos: ">= :monterey"

  app "QPack.app"

  postflight do
    # 清除下载隔离属性并刷新文件关联，避免未签名应用首次打开弹窗拦截
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/QPack.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/com.qpack.app",
    "~/Library/Caches/com.qpack.app",
    "~/Library/Preferences/com.qpack.app.plist",
    "~/Library/Services/使用 QPack 智能解压.workflow",
    "~/Library/Services/使用 QPack 压缩为 ZIP.workflow",
  ]
end
