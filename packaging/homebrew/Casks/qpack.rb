cask "qpack" do
  arch arm: "aarch64", intel: "x64"

  on_arm do
    version "0.1.7"
    sha256 "367f3f422591277fb5b17828cba66f4c12a1581dec160f5638bfe2c479ae7d0f"
  end
  on_intel do
    version "0.1.2"
    sha256 "49a9eda63e730e86550275c3e36cfd26a266decbeed2c32f4d4e072ee950ec58"
  end

  url "http://192.168.31.147:8080/releases/QPack_#{version}_aarch64.dmg"
  name "QPack"
  desc "带中文编码检测与智能解压的压缩工具"
  homepage "http://192.168.31.147:8080"

  depends_on macos: :monterey

  app "QPack.app"

  zap trash: [
    "~/Library/Application Support/com.qpack.app",
    "~/Library/Preferences/com.qpack.app.plist",
    "~/Library/Caches/com.qpack.app",
    "~/Library/Services/使用 QPack 智能解压.workflow",
    "~/Library/Services/使用 QPack 快速压缩.workflow",
  ]
end
