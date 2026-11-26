#!/usr/bin/env node
/**
 * 自动收集已签名的 macOS / Windows 更新产物，复制到 website/releases/ 并生成 Tauri v2 标准 latest.json
 *
 * 用法：
 *   node scripts/generate-latest-json.mjs --base-url http://192.168.31.147:8080/releases --notes "修复双击预览与新增自动更新"
 */
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

function sha256File(filePath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(filePath));
  return hash.digest("hex");
}

const rootDir = process.cwd();
const pkg = JSON.parse(fs.readFileSync(path.join(rootDir, "package.json"), "utf8"));
const version = pkg.version;

const args = process.argv.slice(2);
function getArg(name, fallback) {
  const idx = args.indexOf(name);
  return idx !== -1 && args[idx + 1] ? args[idx + 1] : fallback;
}

const baseUrl = getArg("--base-url", "http://192.168.31.147:8080/releases").replace(/\/$/, "");
const notes = getArg("--notes", `QPack v${version} 更新`);

const releasesDir = path.join(rootDir, "website", "releases");
fs.mkdirSync(releasesDir, { recursive: true });

const artifacts = [
  {
    platform: "darwin-aarch64",
    file: path.join(rootDir, "src-tauri/target/release/bundle/macos/QPack.app.tar.gz"),
    outName: `QPack_${version}_aarch64.app.tar.gz`,
  },
  {
    platform: "windows-x86_64",
    file: path.join(
      rootDir,
      `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/QPack_${version}_x64-setup.exe`
    ),
    outName: `QPack_${version}_x64-setup.exe`,
  },
];

const platforms = {};

for (const item of artifacts) {
  const sigFile = `${item.file}.sig`;
  if (fs.existsSync(item.file) && fs.existsSync(sigFile)) {
    const destPath = path.join(releasesDir, item.outName);
    fs.copyFileSync(item.file, destPath);
    const signature = fs.readFileSync(sigFile, "utf8").trim();
    platforms[item.platform] = {
      signature,
      url: `${baseUrl}/${item.outName}`,
    };
    console.log(`✓ 已收录 ${item.platform}: ${item.outName}`);
  } else {
    console.log(`- 跳过 ${item.platform}（未找到产物或 .sig 签名文件）`);
  }
}

const latestJson = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
};

const outJsonPath = path.join(releasesDir, "latest.json");
fs.writeFileSync(outJsonPath, JSON.stringify(latestJson, null, 2) + "\n", "utf8");
console.log(`\n>>> 已生成更新清单: ${outJsonPath}`);

// 同步更新 Homebrew Cask (macOS) 与 Scoop Manifest (Windows) 的版本号、地址与 SHA256
const siteRootUrl = baseUrl.replace(/\/releases$/, "");
const dmgPath = path.join(releasesDir, `QPack_${version}_aarch64.dmg`);
if (fs.existsSync(dmgPath)) {
  const dmgSha = sha256File(dmgPath);
  const caskPath = path.join(rootDir, "packaging/homebrew/Casks/qpack.rb");
  if (fs.existsSync(caskPath)) {
    let cask = fs.readFileSync(caskPath, "utf8");
    cask = cask
      .replace(/version ".*?"/, `version "${version}"`)
      .replace(/sha256 ".*?"/, `sha256 "${dmgSha}"`)
      .replace(/url ".*?"/, `url "${baseUrl}/QPack_#{version}_aarch64.dmg"`)
      .replace(/homepage ".*?"/, `homepage "${siteRootUrl}"`);
    fs.writeFileSync(caskPath, cask, "utf8");
    console.log(`✓ 已同步更新 Homebrew Cask: ${caskPath}`);
  }
}

const winExePath = path.join(releasesDir, `QPack_${version}_x64-setup.exe`);
if (fs.existsSync(winExePath)) {
  const winSha = sha256File(winExePath);
  const scoopPath = path.join(rootDir, "packaging/scoop/qpack.json");
  if (fs.existsSync(scoopPath)) {
    const scoop = JSON.parse(fs.readFileSync(scoopPath, "utf8"));
    scoop.version = version;
    scoop.homepage = siteRootUrl;
    scoop.architecture["64bit"].url = `${baseUrl}/QPack_${version}_x64-setup.exe#/dl.exe`;
    scoop.architecture["64bit"].hash = winSha;
    fs.writeFileSync(scoopPath, JSON.stringify(scoop, null, 2) + "\n", "utf8");
    console.log(`✓ 已同步更新 Scoop Manifest: ${scoopPath}`);
  }
}
