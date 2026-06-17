# cli-or-gui

[![Crates.io](https://img.shields.io/crates/v/cli-or-gui)](https://crates.io/crates/cli-or-gui)
[![Docs.rs](https://docs.rs/cli-or-gui/badge.svg)](https://docs.rs/cli-or-gui)
[![License](https://img.shields.io/crates/l/cli-or-gui)](https://spdx.org/licenses/MIT)

A lightweight, zero-bloat, cross-platform Rust crate to safely determine if your application was launched from a CLI terminal (like PowerShell, CMD, Bash) or directly from a graphical interface (like double-clicking in Windows Explorer or launching via a macOS/Linux desktop entry).

It is designed for building single-binary hybrid apps that run as interactive CLI tools when invoked in a terminal, but gracefully initialize a GUI (and hide the default terminal window on Windows) when double-clicked.

---

## Why `cli-or-gui`? (How it compares to alternatives)

Several existing crates attempt to detect if an application was launched from Windows Explorer, but they often fall short in real-world scenarios. Here is how `cli-or-gui` addresses common limitations:

### 1. Robustness Against Intermediate Runners & Wrappers

- **The Problem:** Crates like `crabtrap` or `hide_console_ng` only check if your _immediate_ parent process is `explorer.exe`. If you use packaging tools like `cargo-multivers`, execute via `cargo run`, run inside an IDE runner, or launch through a shell script, the immediate parent is the runner—not the shell or Explorer. This causes those crates to misidentify the launch environment.
- **Our Solution:** `cli-or-gui` uses a climbing ancestor tree-walker. It climbs the process hierarchy past intermediate runners or wrappers until it encounters a recognized terminal/shell (e.g., `cmd.exe`, `powershell.exe`, `pwsh.exe`, `wt.exe`) or the graphical shell (`explorer.exe`), ensuring highly reliable detection.

### 2. Protection Against PID Recycling

- **The Problem:** Windows aggressively recycles Process IDs (PIDs). If an intermediate launcher process starts your app and exits immediately, its PID is freed. The OS can instantly assign that PID to an unrelated active background process (like Chrome or a system service). A simple process walk will follow this recycled PID, leading to false detections or errors.
- **Our Solution:** Every step of our process tree walk is validated by checking the parent process's creation time against the child's. If a parent PID was recycled, the mismatch is caught and the traversal stops.

### 3. Unified, Safe, and Leak-Free API

- **The Problem:** Many helper snippets floating around in the community leak Win32 handles (such as process tokens) or fail to compile on macOS/Linux due to tight coupling with Windows-only types (like `windows::core::Result`).
- **Our Solution:** We provide uniform, safe, platform-agnostic function signatures. On Unix and macOS, the code falls back gracefully to standard POSIX syscalls via `libc` or standard library utilities, preventing cross-compilation headaches. All internal Windows handles are closed properly in every execution path.

### 4. Built-in Caching

- **The Problem:** Taking Win32 process snapshots is a relatively slow operation (taking several milliseconds). Re-querying the environment multiple times is inefficient.
- **Our Solution:** The launch environment is immutable once the application starts. `is_launched_from_terminal()` is cached internally via `OnceLock`. The first call performs the robust check, and subsequent calls return in $O(1)$ time.

---

## Features

- **Terminal Detection:** Safely determines whether stdin is a TTY (Unix) or walks the ancestor tree to find shell environments (Windows).
- **Elevation / Root Check:** Cross-platform privilege check. Detects UAC administrative privilege on Windows, and effective root user ID (`geteuid() == 0`) on Unix/macOS.
- **Console Control:** Hides the console window on Windows when launched as a GUI app, while acting as a silent, compile-friendly no-op on non-Windows platforms.

These features tend to pair well together.

---

## Usage

Add `cli-or-gui` to your `Cargo.toml`:

```text
cargo add cli-or-gui
```

### Basic Example

```rust
fn main() {
    if cli_or_gui::is_launched_from_terminal() {
        // Run as a CLI application
        println!("Running in CLI mode!");
        if cli_or_gui::is_elevated() {
            println!("Elevated privileges detected.");
        }
    } else {
        // Run as a GUI application (e.g., if double-clicked)
        cli_or_gui::hide_console_window();

        // Start your GUI event loop here (eg. druid, egui, slint)
    }
}
```

## Crucial: Configuring Your Windows Subsystem

To build a true hybrid CLI/GUI application, **do NOT** include `#![windows_subsystem = "windows"]` at the top of your `main.rs`.

### Why?

- If you set `#![windows_subsystem = "windows"]`, Windows compiles your app as a pure GUI app. Standard outputs (`println!`) are discarded, and terminal prompts will immediately return before your app finishes running, breaking terminal usage.
- By leaving it unset (defaulting to the Console subsystem), standard CLI pipes and shell waiting work natively.

The `cli-or-gui` crate replaces the need for `#![windows_subsystem = "windows"]` by automatically hiding the console window whenever it detects the application was double-clicked or run from the GUI.

> **Note on Windows "Console Flash":** Because Windows creates the console window _before_ your Rust code is loaded and executed, there may be a very brief terminal window "flash" on double-click before `hide_console_window()` is executed on startup. This is an unavoidable OS-level design behavior for hybrid binaries on Windows.

---

## Platform Support

- **Windows:** Fully supported using modern raw pointer patterns (`&raw mut`) and the official `windows` crate.
- **macOS & Linux:** Fully supported. Uses standard `libc` for POSIX-compliant checks. macOS is covered with no additional hacks required.

---

## License

This project is licensed under the MIT License - see the LICENSE file for details.
