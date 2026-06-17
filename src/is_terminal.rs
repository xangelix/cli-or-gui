use std::sync::OnceLock;

/// Checks if the current application was started from an active CLI terminal.
///
/// This result is cached internally upon the first call, making all subsequent calls O(1).
pub fn is_launched_from_terminal() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(is_launched_from_terminal_inner)
}

#[cfg(not(target_os = "windows"))]
fn is_launched_from_terminal_inner() -> bool {
    use std::io::IsTerminal as _;

    std::io::stdin().is_terminal()
}

#[cfg(target_os = "windows")]
fn is_launched_from_terminal_inner() -> bool {
    use std::collections::HashMap;

    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            Console::GetConsoleWindow,
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            ProcessStatus::K32GetProcessImageFileNameW,
            Threading::{GetCurrentProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
    };

    // 1. Fast-Path: No console allocated? Definitely GUI.
    let console_window = unsafe { GetConsoleWindow() };
    if console_window.0.is_null() {
        return false;
    }

    // 2. Take a snapshot of running processes
    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(handle) if !handle.is_invalid() => handle,
        _ => return true, // Default to true if snapshot fails
    };

    let mut parent_map = HashMap::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    // Build the parent process ID map once
    unsafe {
        if Process32FirstW(snapshot, &raw mut entry).is_ok() {
            loop {
                parent_map.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if Process32NextW(snapshot, &raw mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }

    let mut process_id = unsafe { GetCurrentProcessId() };

    // 3. Walk up the parent chain with no arbitrary limit, validated by creation times
    while let Some(&parent_pid) = parent_map.get(&process_id) {
        if parent_pid == 0 {
            break;
        }

        // Validate the parent is real and not a recycled PID
        if !is_parent_time_valid(process_id, parent_pid) {
            return false; // Parent is dead, this is not an active terminal session
        }

        let Ok(parent_handle) =
            (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid) })
        else {
            return false;
        };

        // Device path suffix matching
        let mut image_file_name = [0u16; 260];

        // Combined path query and CloseHandle into a single unsafe scope
        // to prevent use-after-close of the parent_handle
        let len = unsafe {
            let copied_len = K32GetProcessImageFileNameW(parent_handle, &mut image_file_name);
            let _ = CloseHandle(parent_handle);
            std::cmp::min(copied_len as usize, 260) // Clamp to buffer size
        };

        let process_name = String::from_utf16_lossy(&image_file_name[..len]).to_lowercase();

        if process_name.is_empty() {
            process_id = parent_pid;
            continue;
        }

        if process_name.ends_with("explorer.exe") {
            return false;
        }

        if process_name.ends_with("powershell.exe")
            || process_name.ends_with("cmd.exe")
            || process_name.ends_with("pwsh.exe")
            || process_name.ends_with("wt.exe")
        {
            return true;
        }

        // Keep climbing
        process_id = parent_pid;
    }

    true
}

/// Helper to ensure the parent was created before the child (defends against recycled PIDs)
#[cfg(target_os = "windows")]
fn is_parent_time_valid(child_pid: u32, parent_pid: u32) -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    unsafe {
        let child_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, child_pid);
        let parent_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid);

        let mut valid = false;

        // Consuming both handles in the match pattern ensures we close only what successfully opened.
        match (child_handle, parent_handle) {
            (Ok(c_h), Ok(p_h)) => {
                let mut c_created = FILETIME::default();
                let mut dummy = FILETIME::default();
                let mut p_created = FILETIME::default();

                let c_ok = GetProcessTimes(c_h, &mut c_created, &mut dummy, &mut dummy, &mut dummy)
                    .is_ok();
                let p_ok = GetProcessTimes(p_h, &mut p_created, &mut dummy, &mut dummy, &mut dummy)
                    .is_ok();

                if c_ok && p_ok {
                    let child_time = ((c_created.dwHighDateTime as u64) << 32)
                        | (c_created.dwLowDateTime as u64);
                    let parent_time = ((p_created.dwHighDateTime as u64) << 32)
                        | (p_created.dwLowDateTime as u64);

                    if parent_time < child_time {
                        valid = true;
                    }
                }
                let _ = CloseHandle(c_h);
                let _ = CloseHandle(p_h);
            }
            (Ok(c_h), Err(_)) => {
                let _ = CloseHandle(c_h);
            }
            (Err(_), Ok(p_h)) => {
                let _ = CloseHandle(p_h);
            }
            (Err(_), Err(_)) => {}
        }

        valid
    }
}
