//! QPack Windows 11 原生现代一级级联右键菜单 COM Shell Extension (IExplorerCommand)
//!
//! 仿照 Bandizip 交互：通过单个级联父菜单「QPack」展开上下文子命令：
//! - 选中压缩包时（CLSID_QPACK_EXTRACT）：
//!   1. 解压到此处 (--extract-here)
//!   2. 智能解压到此处 (--extract-smart)
//!   3. 解压到 <压缩包名>\ (--extract-subfolder)
//!   4. 解压到其他文件夹... (--extract-to)
//!   5. 用 QPack 打开... (直接打开)
//! - 选中普通文件/文件夹时（CLSID_QPACK_COMPRESS）：
//!   1. 压缩为 "<名称>.zip" (--compress-zip)
//!   2. 压缩为 "<名称>.7z" (--compress-7z)
//!   3. 添加到压缩包... (--compress-dialog)

#![cfg(windows)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use windows::core::{implement, IUnknown, Interface, Ref, BOOL, GUID, HRESULT, PWSTR};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_NOINTERFACE, HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, IBindCtx, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl,
    IShellItemArray, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED, ECS_HIDDEN, SIGDN_FILESYSPATH,
};

/// 压缩级联菜单 CLSID（对应 AppxManifest.xml 中 * 与 Directory）
pub const CLSID_QPACK_COMPRESS: GUID = GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3a01);

/// 解压级联菜单 CLSID（对应 AppxManifest.xml 中各类压缩包扩展名）
pub const CLSID_QPACK_EXTRACT: GUID = GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3a02);

static DLL_HMODULE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
enum RootKind {
    Compress,
    Extract,
}

#[derive(Clone, Copy)]
enum SubAction {
    ExtractHere,
    ExtractSmart,
    ExtractSubfolder,
    ExtractTo,
    OpenArchive,
    CompressZip,
    Compress7z,
    CompressDialog,
}

impl SubAction {
    fn cli_flag(self) -> Option<&'static str> {
        match self {
            SubAction::ExtractHere => Some("--extract-here"),
            SubAction::ExtractSmart => Some("--extract-smart"),
            SubAction::ExtractSubfolder => Some("--extract-subfolder"),
            SubAction::ExtractTo => Some("--extract-to"),
            SubAction::OpenArchive => None,
            SubAction::CompressZip => Some("--compress-zip"),
            SubAction::Compress7z => Some("--compress-7z"),
            SubAction::CompressDialog => Some("--compress-dialog"),
        }
    }

    fn canonical_guid(self) -> GUID {
        match self {
            SubAction::ExtractHere => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b01),
            SubAction::ExtractSmart => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b02),
            SubAction::ExtractSubfolder => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b03),
            SubAction::ExtractTo => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b04),
            SubAction::OpenArchive => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b05),
            SubAction::CompressZip => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b06),
            SubAction::Compress7z => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b07),
            SubAction::CompressDialog => GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3b08),
        }
    }

    fn title_for(self, selection: Option<&IShellItemArray>) -> String {
        let paths = collect_paths(selection);
        let target_stem = derive_target_stem(&paths);

        match self {
            SubAction::ExtractHere => "解压到此处".into(),
            SubAction::ExtractSmart => "智能解压到此处".into(),
            SubAction::ExtractSubfolder => {
                if let Some(stem) = target_stem {
                    format!("解压到 {stem}\\")
                } else {
                    "解压到同名文件夹".into()
                }
            }
            SubAction::ExtractTo => "解压到其他文件夹...".into(),
            SubAction::OpenArchive => "用 QPack 打开...".into(),
            SubAction::CompressZip => {
                if let Some(stem) = target_stem {
                    format!("压缩为 \"{stem}.zip\"")
                } else {
                    "压缩为 ZIP".into()
                }
            }
            SubAction::Compress7z => {
                if let Some(stem) = target_stem {
                    format!("压缩为 \"{stem}.7z\"")
                } else {
                    "压缩为 7Z".into()
                }
            }
            SubAction::CompressDialog => "添加到压缩包...".into(),
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            SubAction::ExtractHere => "直接解压到当前目录",
            SubAction::ExtractSmart => "智能判断是否创建文件夹，防止解压散落一地或套娃",
            SubAction::ExtractSubfolder => "创建与压缩包同名的文件夹并解压到其中",
            SubAction::ExtractTo => "手动选择目标文件夹进行解压",
            SubAction::OpenArchive => "使用 QPack 打开并预览压缩包内容",
            SubAction::CompressZip => "将选中项快速压缩为 ZIP",
            SubAction::Compress7z => "将选中项快速压缩为高压缩率 7Z",
            SubAction::CompressDialog => "打开 QPack 压缩选项窗口（自定义格式、等级与加密密码）",
        }
    }
}

/// 判断路径是否为受支持的压缩包扩展名
fn is_archive_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    const EXTS: &[&str] = &[
        ".zip", ".7z", ".rar", ".tar", ".gz", ".tgz", ".bz2", ".tbz2", ".xz", ".txz", ".zst",
        ".tzst", ".iso", ".cab", ".dmg", ".wim", ".esd", ".rpm", ".deb", ".lzh", ".lha", ".arj",
        ".cpio",
    ];
    EXTS.iter().any(|ext| lower.ends_with(ext))
}

/// 剥离压缩包后缀（支持双后缀 .tar.gz / .tar.zst 等）
fn strip_archive_ext(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    for ext in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"] {
        if lower.ends_with(ext) && file_name.len() > ext.len() {
            return file_name[..file_name.len() - ext.len()].to_string();
        }
    }
    Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(file_name)
        .to_string()
}

/// 推导右键菜单动态显示的建议名称
fn derive_target_stem(paths: &[String]) -> Option<String> {
    let first = paths.first()?;
    let p = Path::new(first);
    if paths.len() == 1 {
        let name = p.file_name()?.to_str()?;
        if p.is_dir() {
            Some(name.to_string())
        } else {
            Some(strip_archive_ext(name))
        }
    } else {
        p.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
    }
}

/// 将 Rust &str 分配为 COM 要求的 CoTaskMem UTF-16 字符串
fn alloc_co_task_pwstr(s: &str) -> windows::core::Result<PWSTR> {
    let wide: Vec<u16> = std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    unsafe {
        let ptr = CoTaskMemAlloc(byte_len) as *mut u16;
        if ptr.is_null() {
            return Err(windows::core::Error::from_win32());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        Ok(PWSTR(ptr))
    }
}

/// 定位与当前 DLL 同目录的 qpack.exe
fn resolve_qpack_exe() -> Option<PathBuf> {
    let raw = DLL_HMODULE.load(Ordering::Relaxed);
    let hmod = HMODULE(raw as *mut c_void);
    let mut buf = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(Some(hmod), &mut buf) } as usize;
    if len == 0 || len >= buf.len() {
        return None;
    }
    let dll_path = PathBuf::from(String::from_utf16_lossy(&buf[..len]));
    let dir = dll_path.parent()?;
    let exe = dir.join("qpack.exe");
    if exe.exists() {
        Some(exe)
    } else {
        None
    }
}

/// 从 IShellItemArray 提取所有选中文件/文件夹的本地路径
fn collect_paths(selection: Option<&IShellItemArray>) -> Vec<String> {
    let Some(arr) = selection else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    unsafe {
        if let Ok(count) = arr.GetCount() {
            for i in 0..count {
                if let Ok(item) = arr.GetItemAt(i) {
                    if let Ok(pwstr) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                        if let Ok(path_str) = pwstr.to_string() {
                            if !path_str.is_empty() {
                                paths.push(path_str);
                            }
                        }
                        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const c_void));
                    }
                }
            }
        }
    }
    paths
}

fn spawn_qpack(flag: Option<&str>, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    if let Some(exe) = resolve_qpack_exe() {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = Command::new(exe);
        if let Some(f) = flag {
            cmd.arg(f);
        }
        let _ = cmd.args(paths).creation_flags(CREATE_NO_WINDOW).spawn();
    }
}

fn exe_icon_pwstr() -> windows::core::Result<PWSTR> {
    if let Some(exe) = resolve_qpack_exe() {
        alloc_co_task_pwstr(&format!("{},0", exe.to_string_lossy()))
    } else {
        Err(windows::core::Error::from(S_FALSE))
    }
}

// ============================================================================
// 叶子子命令 (IExplorerCommand)
// ============================================================================

#[implement(IExplorerCommand)]
struct QPackSubCommand {
    action: SubAction,
}

impl IExplorerCommand_Impl for QPackSubCommand_Impl {
    fn GetTitle(&self, psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr(&self.action.title_for(psiitemarray.as_ref()))
    }

    fn GetIcon(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        exe_icon_pwstr()
    }

    fn GetToolTip(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr(self.action.tooltip())
    }

    fn GetCanonicalName(&self) -> windows::core::Result<GUID> {
        Ok(self.action.canonical_guid())
    }

    fn GetState(
        &self,
        _psiitemarray: Ref<'_, IShellItemArray>,
        _foktobeslow: BOOL,
    ) -> windows::core::Result<u32> {
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(
        &self,
        psiitemarray: Ref<'_, IShellItemArray>,
        _pbc: Ref<'_, IBindCtx>,
    ) -> windows::core::Result<()> {
        let paths = collect_paths(psiitemarray.as_ref());
        spawn_qpack(self.action.cli_flag(), &paths);
        Ok(())
    }

    fn GetFlags(&self) -> windows::core::Result<u32> {
        Ok(ECF_DEFAULT.0 as u32)
    }

    fn EnumSubCommands(&self) -> windows::core::Result<IEnumExplorerCommand> {
        Err(windows::core::Error::from(E_NOINTERFACE))
    }
}

// ============================================================================
// 子命令枚举器 (IEnumExplorerCommand)
// ============================================================================

#[implement(IEnumExplorerCommand)]
struct QPackEnumExplorerCommand {
    commands: Vec<IExplorerCommand>,
    index: Mutex<usize>,
}

impl IEnumExplorerCommand_Impl for QPackEnumExplorerCommand_Impl {
    fn Next(
        &self,
        celt: u32,
        pcommands: *mut Option<IExplorerCommand>,
        pceltfetched: *mut u32,
    ) -> HRESULT {
        if pcommands.is_null() {
            return HRESULT(-2147024809); // E_INVALIDARG
        }
        let mut idx = self.index.lock().unwrap();
        let mut fetched = 0u32;

        while fetched < celt && *idx < self.commands.len() {
            unsafe {
                pcommands
                    .add(fetched as usize)
                    .write(Some(self.commands[*idx].clone()));
            }
            *idx += 1;
            fetched += 1;
        }

        if !pceltfetched.is_null() {
            unsafe {
                *pceltfetched = fetched;
            }
        }

        if fetched == celt {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn Skip(&self, celt: u32) -> windows::core::Result<()> {
        let mut idx = self.index.lock().unwrap();
        let next = idx.saturating_add(celt as usize);
        if next <= self.commands.len() {
            *idx = next;
            Ok(())
        } else {
            *idx = self.commands.len();
            Err(windows::core::Error::from(S_FALSE))
        }
    }

    fn Reset(&self) -> windows::core::Result<()> {
        *self.index.lock().unwrap() = 0;
        Ok(())
    }

    fn Clone(&self) -> windows::core::Result<IEnumExplorerCommand> {
        let cur = *self.index.lock().unwrap();
        Ok(QPackEnumExplorerCommand {
            commands: self.commands.clone(),
            index: Mutex::new(cur),
        }
        .into())
    }
}

// ============================================================================
// 顶层级联父菜单 (IExplorerCommand)
// ============================================================================

#[implement(IExplorerCommand)]
struct QPackRootCommand {
    kind: RootKind,
}

impl IExplorerCommand_Impl for QPackRootCommand_Impl {
    fn GetTitle(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr("QPack")
    }

    fn GetIcon(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        exe_icon_pwstr()
    }

    fn GetToolTip(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr("QPack 解压缩与打包工具")
    }

    fn GetCanonicalName(&self) -> windows::core::Result<GUID> {
        Ok(match self.kind {
            RootKind::Compress => CLSID_QPACK_COMPRESS,
            RootKind::Extract => CLSID_QPACK_EXTRACT,
        })
    }

    fn GetState(
        &self,
        psiitemarray: Ref<'_, IShellItemArray>,
        _foktobeslow: BOOL,
    ) -> windows::core::Result<u32> {
        let paths = collect_paths(psiitemarray.as_ref());
        let all_archives = !paths.is_empty() && paths.iter().all(|p| is_archive_path(p));

        // 选中全部为压缩包时隐藏 Compress 入口，避免右键菜单出现两个重复的「QPack」项
        match self.kind {
            RootKind::Compress if all_archives => Ok(ECS_HIDDEN.0 as u32),
            _ => Ok(ECS_ENABLED.0 as u32),
        }
    }

    fn Invoke(
        &self,
        _psiitemarray: Ref<'_, IShellItemArray>,
        _pbc: Ref<'_, IBindCtx>,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetFlags(&self) -> windows::core::Result<u32> {
        Ok(ECF_HASSUBCOMMANDS.0 as u32)
    }

    fn EnumSubCommands(&self) -> windows::core::Result<IEnumExplorerCommand> {
        let actions: &[SubAction] = match self.kind {
            RootKind::Extract => &[
                SubAction::ExtractHere,
                SubAction::ExtractSmart,
                SubAction::ExtractSubfolder,
                SubAction::ExtractTo,
                SubAction::OpenArchive,
            ],
            RootKind::Compress => &[
                SubAction::CompressZip,
                SubAction::Compress7z,
                SubAction::CompressDialog,
            ],
        };

        let commands: Vec<IExplorerCommand> = actions
            .iter()
            .map(|&action| QPackSubCommand { action }.into())
            .collect();

        Ok(QPackEnumExplorerCommand {
            commands,
            index: Mutex::new(0),
        }
        .into())
    }
}

#[implement(IClassFactory)]
struct QPackClassFactory {
    kind: RootKind,
}

impl IClassFactory_Impl for QPackClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return Err(windows::core::Error::from(E_NOINTERFACE));
        }
        let cmd: IExplorerCommand = QPackRootCommand { kind: self.kind }.into();
        unsafe { cmd.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    if fdw_reason == DLL_PROCESS_ATTACH {
        DLL_HMODULE.store(hinst_dll.0 as usize, Ordering::Relaxed);
    }
    BOOL(1)
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_NOINTERFACE;
    }
    let clsid = unsafe { *rclsid };
    let kind = if clsid == CLSID_QPACK_COMPRESS {
        RootKind::Compress
    } else if clsid == CLSID_QPACK_EXTRACT {
        RootKind::Extract
    } else {
        return CLASS_E_CLASSNOTAVAILABLE;
    };

    let factory: IClassFactory = QPackClassFactory { kind }.into();
    unsafe { factory.query(riid, ppv) }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}
