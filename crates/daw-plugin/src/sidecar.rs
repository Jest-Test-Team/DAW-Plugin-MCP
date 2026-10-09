//! Locate and spawn the bundled `daw-mcp` sidecar. Editor / worker threads only.
//! Never call from `process()`.

use serde_json::json;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn already_listening() -> bool {
    crate::ipc::rpc_call("plugin/hello", json!({})).is_ok()
}

/// Spawn `daw-mcp --sidecar --host=<saved>` if the socket is down. Does not kill an existing daemon.
pub fn spawn_if_needed(host: &str) -> Result<PathBuf, String> {
    if already_listening() {
        return find_daemon_binary().or_else(|_| Ok(PathBuf::from("daw-mcp")));
    }
    let host = if host.trim().is_empty() {
        "logic"
    } else {
        host.trim()
    };
    let bin = find_daemon_binary()?;
    let mut cmd = Command::new(&bin);
    cmd.arg("--sidecar")
        .arg(format!("--host={host}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach so closing the editor does not take the sidecar with it.
        cmd.process_group(0);
    }
    cmd.spawn()
        .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
    Ok(bin)
}

pub fn wait_until_up(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if already_listening() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    already_listening()
}

pub fn find_daemon_binary() -> Result<PathBuf, String> {
    let tried = daemon_candidates();
    for p in &tried {
        if p.is_file() {
            return Ok(p.clone());
        }
    }
    let list = tried
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    if list.is_empty() {
        Err("找不到 daw-mcp（bundle Contents/MacOS 沒有 sidecar）".into())
    } else {
        Err(format!("找不到 daw-mcp：{list}"))
    }
}

pub fn daemon_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dylib) = this_plugin_path() {
        if let Some(parent) = dylib.parent() {
            push_unique(&mut out, parent.join(daemon_name()));
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let crate_dir = PathBuf::from(manifest);
        if let Some(ws) = crate_dir.parent().and_then(|p| p.parent()) {
            push_unique(&mut out, ws.join("target/release").join(daemon_name()));
            push_unique(&mut out, ws.join("target/debug").join(daemon_name()));
        }
    }
    out
}

fn daemon_name() -> &'static str {
    if cfg!(windows) {
        "daw-mcp.exe"
    } else {
        "daw-mcp"
    }
}

fn push_unique(out: &mut Vec<PathBuf>, path: PathBuf) {
    if !out.contains(&path) {
        out.push(path);
    }
}

fn this_plugin_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        dladdr_self()
    }
    #[cfg(windows)]
    {
        win_module_path()
    }
}

/// `current_exe()` inside an AU is Logic, not the plugin. Resolve this dylib via dladdr.
#[cfg(unix)]
fn dladdr_self() -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int, c_void};

    #[repr(C)]
    struct DlInfo {
        dli_fname: *const c_char,
        dli_fbase: *mut c_void,
        dli_sname: *const c_char,
        dli_saddr: *mut c_void,
    }

    extern "C" {
        fn dladdr(addr: *const c_void, info: *mut DlInfo) -> c_int;
    }

    unsafe {
        let mut info = std::mem::zeroed::<DlInfo>();
        if dladdr(dladdr_self as *const c_void, &mut info) == 0 || info.dli_fname.is_null() {
            return None;
        }
        let s = CStr::from_ptr(info.dli_fname).to_str().ok()?;
        Some(PathBuf::from(s))
    }
}

#[cfg(windows)]
fn win_module_path() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;

    extern "system" {
        fn GetModuleHandleExW(
            dw_flags: u32,
            lp_module_name: *const u16,
            ph_module: *mut *mut core::ffi::c_void,
        ) -> i32;
        fn GetModuleFileNameW(
            h_module: *mut core::ffi::c_void,
            lp_filename: *mut u16,
            n_size: u32,
        ) -> u32;
    }

    const FROM_ADDRESS: u32 = 0x0000_0004;
    const UNCHANGED_REFCOUNT: u32 = 0x0000_0002;

    unsafe {
        let mut handle = std::ptr::null_mut();
        let ok = GetModuleHandleExW(
            FROM_ADDRESS | UNCHANGED_REFCOUNT,
            win_module_path as *const u16,
            &mut handle,
        );
        if ok == 0 || handle.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let n = GetModuleFileNameW(handle, buf.as_mut_ptr(), buf.len() as u32);
        if n == 0 {
            return None;
        }
        Some(PathBuf::from(std::ffi::OsString::from_wide(&buf[..n as usize])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_name_matches_platform() {
        let n = daemon_name();
        assert!(n == "daw-mcp" || n == "daw-mcp.exe");
    }

    #[test]
    fn candidates_name_sidecar() {
        let c = daemon_candidates();
        assert!(c.iter().any(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("daw-mcp"))
        }));
    }
}
