#[cfg(target_os = "windows")]
pub fn relaunch_as_elevated() -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;

    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWDEFAULT},
        core::PCWSTR,
    };

    let current_exe = std::env::current_exe()?;
    let mut exe_path: Vec<u16> = current_exe.as_os_str().encode_wide().collect();
    exe_path.push(0);

    let mut verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().collect();
    verb.push(0);

    // Reconstruct the command-line arguments to pass to the elevated process.
    // Windows arguments containing spaces must be enclosed in double quotes.
    let mut parameters_str = String::new();
    for arg in std::env::args().skip(1) {
        if !parameters_str.is_empty() {
            parameters_str.push(' ');
        }
        if arg.contains(' ') || arg.contains('\t') {
            parameters_str.push('"');
            parameters_str.push_str(&arg.replace('"', "\\\""));
            parameters_str.push('"');
        } else {
            parameters_str.push_str(&arg);
        }
    }

    let mut parameters: Vec<u16> = std::ffi::OsString::from(parameters_str)
        .as_os_str()
        .encode_wide()
        .collect();
    parameters.push(0);

    let lp_parameters = if parameters.len() > 1 {
        PCWSTR::from_raw(parameters.as_ptr())
    } else {
        PCWSTR::null()
    };

    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::from_raw(verb.as_ptr()),
            PCWSTR::from_raw(exe_path.as_ptr()),
            lp_parameters,
            None,
            SW_SHOWDEFAULT,
        )
    };

    // If ShellExecuteW returns a value <= 32, it represents an error.
    let status = result.0 as usize;
    if status <= 32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("ShellExecuteW failed with error code: {status}"),
        ));
    }

    // Exit the current process as it has successfully spawned its elevated counterpart.
    std::process::exit(0);
}

#[cfg(not(target_os = "windows"))]
pub fn relaunch_as_elevated() -> std::io::Result<()> {
    use std::{os::unix::process::CommandExt as _, process::Command};

    let current_exe = std::env::current_exe()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    // Re-execute the current process under sudo.
    // .exec() only returns if it fails to execute (e.g., sudo is missing).
    let err = Command::new("sudo").arg(current_exe).args(&args).exec();

    Err(err)
}
