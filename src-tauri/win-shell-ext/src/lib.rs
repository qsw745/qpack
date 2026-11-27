//! QPack Windows 11 原生现代一级右键菜单 COM Shell Extension (IExplorerCommand)
//!
//! 注册两个一级右键菜单项（通过 MSIX AppxManifest.xml 声明）：
//! - CLSID_COMPRESS: 「使用 QPack 压缩为 ZIP」（所有文件与文件夹）
//! - CLSID_EXTRACT:  「使用 QPack 智能解压」（支持的压缩包格式）

#![cfg(windows)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{implement, IUnknown, Interface, Ref, BOOL, GUID, HRESULT, PWSTR};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_NOINTERFACE, HINSTANCE, HMODULE, S_FALSE,
};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, IBindCtx, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
    ECF_DEFAULT, ECS_ENABLED, SIGDN_FILESYSPATH,
};

/// 「使用 QPack 压缩为 ZIP」CLSID
pub const CLSID_QPACK_COMPRESS: GUID = GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3a01);

/// 「使用 QPack 智能解压」CLSID
pub const CLSID_QPACK_EXTRACT: GUID = GUID::from_u128(0x8b6a9c10_5e42_4d31_9a77_2c6f814e3a02);

static DLL_HMODULE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
enum ActionKind {
    Compress,
    Extract,
}

impl ActionKind {
    fn title(self) -> &'static str {
        match self {
            ActionKind::Compress => "使用 QPack 压缩为 ZIP",
            ActionKind::Extract => "使用 QPack 智能解压",
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            ActionKind::Compress => "使用 QPack 将选中文件或文件夹快速打包为 ZIP",
            ActionKind::Extract => "使用 QPack 智能解压压缩包（防套娃、防乱码）",
        }
    }

    fn cli_flag(self) -> &'static str {
        match self {
            ActionKind::Compress => "--compress",
            ActionKind::Extract => "--extract",
        }
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

#[implement(IExplorerCommand)]
struct QPackExplorerCommand {
    kind: ActionKind,
}

impl IExplorerCommand_Impl for QPackExplorerCommand_Impl {
    fn GetTitle(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr(self.kind.title())
    }

    fn GetIcon(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        if let Some(exe) = resolve_qpack_exe() {
            alloc_co_task_pwstr(&format!("{},0", exe.to_string_lossy()))
        } else {
            Err(windows::core::Error::from(S_FALSE))
        }
    }

    fn GetToolTip(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> windows::core::Result<PWSTR> {
        alloc_co_task_pwstr(self.kind.tooltip())
    }

    fn GetCanonicalName(&self) -> windows::core::Result<GUID> {
        Ok(match self.kind {
            ActionKind::Compress => CLSID_QPACK_COMPRESS,
            ActionKind::Extract => CLSID_QPACK_EXTRACT,
        })
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
        if paths.is_empty() {
            return Ok(());
        }
        if let Some(exe) = resolve_qpack_exe() {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = Command::new(exe)
                .arg(self.kind.cli_flag())
                .args(&paths)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
        Ok(())
    }

    fn GetFlags(&self) -> windows::core::Result<u32> {
        Ok(ECF_DEFAULT.0 as u32)
    }

    fn EnumSubCommands(&self) -> windows::core::Result<IEnumExplorerCommand> {
        Err(windows::core::Error::from(E_NOINTERFACE))
    }
}

#[implement(IClassFactory)]
struct QPackClassFactory {
    kind: ActionKind,
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
        let cmd: IExplorerCommand = QPackExplorerCommand { kind: self.kind }.into();
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
        ActionKind::Compress
    } else if clsid == CLSID_QPACK_EXTRACT {
        ActionKind::Extract
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
