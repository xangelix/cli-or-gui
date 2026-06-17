#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            GetTokenInformation, TOKEN_ACCESS_MASK, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut h_token: HANDLE = HANDLE(0 as _);
    unsafe {
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ACCESS_MASK(TOKEN_QUERY.0),
            &raw mut h_token,
        )
        .is_err()
        {
            return false;
        }

        let mut token_elevation: TOKEN_ELEVATION = core::mem::zeroed();
        let mut return_length = 0;

        let info_result = GetTokenInformation(
            h_token,
            TokenElevation,
            Some((&raw mut token_elevation).cast()),
            core::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &raw mut return_length,
        );

        // Safely close the token handle when done to prevent handle leaks
        let _ = CloseHandle(h_token);

        if info_result.is_ok() {
            token_elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[must_use]
pub fn is_elevated() -> bool {
    // Under Unix/macOS, "elevated" means running as root (effective UID 0)
    unsafe { libc::geteuid() == 0 }
}
