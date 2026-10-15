# green-state

A single-file Windows tray app that keeps your Microsoft Teams status green.

It sits in the system tray as a little energy-drink can with a gem in the corner. While enabled it blocks sleep and
display-off, and whenever you have been idle for a minute it injects input nothing reacts to
(a one-pixel mouse move and back, plus an F15 key release) so Windows never reports you as idle.
Nothing is injected while you are actually using the machine.

Left-click the can to switch it on or off; it turns grey whenever it is not keeping you green.
The right-click menu has:

- **Keep Teams green**: the same on/off switch, remembered across restarts.
- **Pause**: 15 minutes, 30 minutes, 1 hour, 2 hours or the rest of the day, or resume early.
- **Work hours**: only stay green Monday to Friday 8–17, 9–18 or 8–18 (or any time, the default).
- **Start with Windows**: launch at sign-in.

While the session is locked the app stops holding the screen on (it still keeps the machine
awake), so the monitor turns off as usual on the lock screen.

Settings live in `HKCU\Software\green-state`.

## Build

On Windows:

    cargo build --release

Cross-compiling from Linux/WSL with [cargo-xwin](https://github.com/rust-cross/cargo-xwin):

    cargo xwin build --release --target x86_64-pc-windows-msvc

The result is one self-contained `green-state.exe` (static CRT, icon embedded).

## Notes

- Windows 11 puts new tray icons in the hidden-icons flyout (the `^` next to the clock); drag the
  can onto the taskbar to keep it visible.
- Windows rejects injected input while the session is locked, so the app cannot help there (Teams
  shows *Away* on a locked session regardless). It stands by until you unlock.
- The same happens in a Remote Desktop session while the client window is minimised. The usual
  workaround is on the *client* machine: create the DWORD `RemoteDesktop_SuppressWhenMinimized` = `2`
  under `HKCU\Software\Microsoft\Terminal Server Client` and reconnect.
