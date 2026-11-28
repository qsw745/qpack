/**
 * QPack 官网发布配置（发版时只需更新版本号与对应安装包候选路径）
 */
const RELEASE_CONFIG = {
  version: "v0.1.7",
  downloads: {
    "macos-arm64": {
      osName: "macOS (Apple Silicon)",
      label: "下载 macOS (Apple Silicon)",
      candidates: [
        "./releases/QPack_0.1.7_aarch64.dmg",
        "./releases/QPack_0.1.6_aarch64.dmg",
      ],
    },
    "macos-x64": {
      osName: "macOS (Intel)",
      label: "下载 macOS (Intel)",
      candidates: [
        "./releases/QPack_0.1.7_x64.dmg",
        "./releases/QPack_0.1.6_x64.dmg",
        "./releases/QPack_0.1.2_x64.dmg",
      ],
    },
    "windows-x64": {
      osName: "Windows 64 位",
      label: "下载 Windows (64 位)",
      candidates: [
        "./releases/QPack_0.1.7_x64-setup.exe",
        "./releases/QPack_0.1.6_x64-setup.exe",
      ],
    },
  },
};

function showToast(message) {
  const toast = document.getElementById("toast");
  if (!toast) return;
  toast.textContent = message;
  toast.classList.add("show");
  clearTimeout(window.__qpackToastTimer);
  window.__qpackToastTimer = setTimeout(() => {
    toast.classList.remove("show");
  }, 2400);
}

function detectPlatform() {
  const ua = navigator.userAgent.toLowerCase();
  const platform = (navigator.platform || "").toLowerCase();
  if (ua.includes("win") || platform.includes("win")) {
    return "windows-x64";
  }
  return "macos-arm64";
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "";
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(0)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

async function resolveAvailableRelease(candidates) {
  for (const url of candidates) {
    try {
      const res = await fetch(url, { method: "HEAD" });
      if (res.ok) {
        const len = Number(res.headers.get("content-length") || 0);
        return { url, sizeBytes: len > 0 ? len : null };
      }
    } catch {
      // 忽略网络错误，尝试下一个候选文件
    }
  }
  return null;
}

function markUnavailable(link, platformKey) {
  const hint =
    platformKey === "windows-x64"
      ? "Windows 安装包暂未上传，请稍后再试"
      : "该芯片架构安装包暂未上传，请先选择其他版本";
  link.dataset.unavailable = "true";
  link.style.opacity = "0.68";
  link.textContent = `${link.textContent.trim()}（待发布）`;
  link.addEventListener("click", (e) => {
    e.preventDefault();
    showToast(hint);
  });
}

function initDownloads() {
  const navVer = document.getElementById("nav-version");
  const dlVer = document.getElementById("dl-version");
  if (navVer) navVer.textContent = RELEASE_CONFIG.version;
  if (dlVer) dlVer.textContent = RELEASE_CONFIG.version;

  const detected = detectPlatform();
  const primaryBtn = document.getElementById("primary-download-btn");
  const primaryLabel = document.getElementById("primary-download-label");
  const detectedHint = document.getElementById("detected-os-hint");
  const primaryInfo = RELEASE_CONFIG.downloads[detected];

  if (primaryLabel && primaryInfo) {
    primaryLabel.textContent = `${primaryInfo.label} (${RELEASE_CONFIG.version})`;
  }
  if (detectedHint && primaryInfo) {
    detectedHint.innerHTML = `已自动识别你的系统：<strong>${primaryInfo.osName}</strong> · <a href="#download">查看其他平台</a>`;
  }

  // 把当前操作系统对应的卡片置顶到第一位，并动态切换为主推荐样式
  const grid = document.getElementById("download-grid");
  const detectedCard = document.querySelector(`.dl-card[data-platform="${detected}"]`);
  if (grid && detectedCard) {
    grid.prepend(detectedCard);
    detectedCard.classList.add("highlighted");
    const head = detectedCard.querySelector(".dl-head");
    if (head && !head.querySelector(".rec-badge")) {
      const badge = document.createElement("span");
      badge.className = "rec-badge";
      badge.textContent = "当前系统推荐";
      head.appendChild(badge);
    }
  }

  Object.entries(RELEASE_CONFIG.downloads).forEach(([key, item]) => {
    const link = document.getElementById(`dl-${key}`);
    if (link) {
      // 仅当前系统的下载按钮用主按钮样式，其余平台用次级按钮
      link.classList.toggle("btn-primary", key === detected);
      link.classList.toggle("btn-secondary", key !== detected);
      link.setAttribute("href", item.candidates[0]);

      resolveAvailableRelease(item.candidates).then((found) => {
        if (found) {
          link.setAttribute("href", found.url);
          const sizeTag = found.sizeBytes ? ` · ${formatBytes(found.sizeBytes)}` : "";
          if (key === "windows-x64") {
            const base = found.url.endsWith(".exe")
              ? "下载 Windows 安装程序 (.exe)"
              : "下载 Windows 安装程序 (.msi)";
            link.textContent = `${base}${sizeTag}`;
          } else {
            link.textContent = `${link.textContent.trim()}${sizeTag}`;
          }

          // 首屏 Hero 主下载按钮直连当前系统的真实安装包与真实大小
          if (key === detected && primaryBtn && primaryLabel) {
            primaryBtn.setAttribute("href", found.url);
            primaryLabel.textContent = `${item.label} ${RELEASE_CONFIG.version}${sizeTag}`;
          }
        } else {
          markUnavailable(link, key);
        }
      });
    }
  });
}

/** 命令行一键安装卡片（支持 macOS curl / Windows PowerShell / Homebrew Cask） */
function initCliInstaller() {
  const cmdEl = document.getElementById("cli-install-cmd");
  const promptEl = document.getElementById("cli-install-prompt");
  const copyBtn = document.getElementById("cli-install-copy");
  const tabs = document.querySelectorAll(".cli-tab");
  if (!cmdEl || !copyBtn || !tabs.length) return;

  const origin =
    window.location.origin && window.location.origin.startsWith("http")
      ? window.location.origin
      : "http://192.168.31.147:8080";

  const commands = {
    macos: { prompt: "$", cmd: `curl -fsSL ${origin}/install.sh | bash` },
    windows: { prompt: "PS>", cmd: `irm ${origin}/install.ps1 | iex` },
    brew: { prompt: "$", cmd: "brew install --cask qsw745/tap/qpack" },
  };

  const setActive = (os) => {
    const entry = commands[os] || commands.macos;
    tabs.forEach((t) => {
      t.classList.toggle("active", t.getAttribute("data-cli-os") === os);
    });
    if (promptEl) promptEl.textContent = entry.prompt;
    cmdEl.textContent = entry.cmd;
    copyBtn.setAttribute("data-copy", entry.cmd);
  };

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      setActive(tab.getAttribute("data-cli-os") || "macos");
    });
  });

  const defaultOs = detectPlatform() === "windows-x64" ? "windows" : "macos";
  setActive(defaultOs);
}

/** 交互式预览窗口：中文编码切换对比 + 智能解压进度演示 */
function initShowcaseDemo() {
  const encBtns = document.querySelectorAll(".enc-btn");
  const nameNodes = document.querySelectorAll(".file-name b");
  const statusEl = document.getElementById("demo-status");
  const progressFill = document.getElementById("demo-progress-fill");

  encBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      const mode = btn.getAttribute("data-enc");
      encBtns.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");

      nameNodes.forEach((node) => {
        const nextText = node.getAttribute(mode === "garbled" ? "data-garbled" : "data-utf8");
        if (nextText) node.textContent = nextText;
      });

      if (statusEl) {
        statusEl.firstElementChild.textContent =
          mode === "garbled"
            ? "传统工具在跨平台打开 GBK 压缩包时的常见乱码效果"
            : "QPack 已自动识别 GBK / UTF-8 编码，中文文件名完美还原";
      }
    });
  });

  const extractBtn = document.getElementById("demo-extract-btn");
  if (extractBtn && statusEl) {
    extractBtn.addEventListener("click", () => {
      if (progressFill) {
        progressFill.style.width = "35%";
        setTimeout(() => { progressFill.style.width = "100%"; }, 140);
        setTimeout(() => { progressFill.style.width = "0%"; }, 900);
      }
      statusEl.firstElementChild.textContent =
        "已智能解压至「2026项目交付资料包/」（自动清理 3 个 .DS_Store / __MACOSX 冗余文件）";
      showToast("智能解压完成：已自动防套娃并过滤系统垃圾文件");
    });
  }
}

/** 一键复制命令行 */
function initCopyButtons() {
  document.querySelectorAll("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const text = btn.getAttribute("data-copy") || "";
      try {
        await navigator.clipboard.writeText(text);
        showToast(`已复制：${text}`);
      } catch {
        showToast("请手动选中命令复制");
      }
    });
  });
}

/** 深浅色主题切换 */
function initTheme() {
  const root = document.documentElement;
  const saved = localStorage.getItem("qpack-website-theme");
  const prefersDark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  root.setAttribute("data-theme", saved || (prefersDark ? "dark" : "light"));

  const toggle = document.getElementById("theme-toggle");
  if (toggle) {
    toggle.addEventListener("click", () => {
      const next = root.getAttribute("data-theme") === "dark" ? "light" : "dark";
      root.setAttribute("data-theme", next);
      localStorage.setItem("qpack-website-theme", next);
    });
  }
}

/** 导航栏滚动微阴影 + 元素平滑入场动画 */
function initScrollEffects() {
  const navbar = document.getElementById("navbar");
  if (navbar) {
    const onScroll = () => navbar.classList.toggle("scrolled", window.scrollY > 12);
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
  }

  const revealEls = document.querySelectorAll(".reveal");
  if (!("IntersectionObserver" in window)) {
    revealEls.forEach((el) => el.classList.add("is-visible"));
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add("is-visible");
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.08, rootMargin: "0px 0px -24px 0px" }
  );
  revealEls.forEach((el) => observer.observe(el));
}

initTheme();
initDownloads();
initCliInstaller();
initShowcaseDemo();
initCopyButtons();
initScrollEffects();
