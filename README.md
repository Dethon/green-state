# green-state

A single-file Windows tray app that keeps your Microsoft Teams status green.

It sits in the system tray as a little energy-drink can. While enabled it blocks sleep and
display-off, and whenever you have been idle for a minute it injects input nothing reacts to
(a one-pixel mouse move and back, plus an F15 key release) so Windows never reports you as idle.
Nothing is injected while you are actually using the machine.

Tray menu: **Keep Teams green** (toggle, the can turns grey when paused) and **Quit**.

## Build

On Windows:

    cargo build --release

Cross-compiling from Linux/WSL with [cargo-xwin](https://github.com/rust-cross/cargo-xwin):

    cargo xwin build --release --target x86_64-pc-windows-msvc

The result is one self-contained `green-state.exe` (static CRT, icon embedded). To start it with
Windows, drop a shortcut to it into `shell:startup`.

## Notes

- Windows 11 puts new tray icons in the hidden-icons flyout (the `^` next to the clock); drag the
  can onto the taskbar to keep it visible.
- Windows rejects injected input while the session is locked, so the app cannot help there (Teams
  shows *Away* on a locked session regardless).
- The same happens in a Remote Desktop session while the client window is minimised. The usual
  workaround is on the *client* machine: create the DWORD `RemoteDesktop_SuppressWhenMinimized` = `2`
  under `HKCU\Software\Microsoft\Terminal Server Client` and reconnect.
