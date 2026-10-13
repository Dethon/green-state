#![windows_subsystem = "windows"]

#[cfg(windows)]
mod icon;

#[cfg(windows)]
mod app {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetLastInputInfo, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        LASTINPUTINFO, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput, VK_F15,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, GetSystemMetrics, MSG, SM_CXSMICON, TranslateMessage,
    };
    use windows_sys::w;

    use crate::icon;

    /// Teams flips to "Away" after five idle minutes; stay far below that.
    const IDLE_LIMIT: Duration = Duration::from_secs(60);
    const POLL_INTERVAL: Duration = Duration::from_secs(15);

    static ACTIVE: AtomicBool = AtomicBool::new(true);

    pub fn run() {
        unsafe {
            CreateMutexW(null(), 0, w!("Local\\green-state-single-instance"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let toggle = CheckMenuItem::new("Keep Teams green", true, true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[&toggle, &PredefinedMenuItem::separator(), &quit])
            .expect("failed to build tray menu");

        // Dropping the tray handle removes the icon, so it lives until `run` returns.
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(true))
            .with_icon(tray_icon(true))
            .build()
            .expect("failed to create tray icon");

        thread::spawn(keep_active);

        unsafe {
            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // Menu events are emitted from the tray's window procedure, i.e. during dispatch.
                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == *quit.id() {
                        return;
                    }
                    if event.id == *toggle.id() {
                        let active = toggle.is_checked();
                        ACTIVE.store(active, Ordering::Relaxed);
                        let _ = tray.set_icon(Some(tray_icon(active)));
                        let _ = tray.set_tooltip(Some(tooltip(active)));
                    }
                }
            }
        }
    }

    fn tooltip(active: bool) -> &'static str {
        if active { "Green State: keeping Teams green" } else { "Green State: paused" }
    }

    fn tray_icon(active: bool) -> Icon {
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
        Icon::from_rgba(icon::render(size, active), size, size).expect("invalid icon buffer")
    }

    /// Blocks sleep and display-off, and fakes input whenever the user has been idle too long.
    /// Real activity is never interfered with: nothing is injected while the user is at the keyboard.
    fn keep_active() {
        loop {
            // The execution state is per thread, so it has to be (re)set from this one.
            if ACTIVE.load(Ordering::Relaxed) {
                unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED) };
                if idle_time() >= IDLE_LIMIT {
                    nudge();
                }
            } else {
                unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn idle_time() -> Duration {
        let mut info = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return Duration::ZERO;
        }
        Duration::from_millis(unsafe { GetTickCount() }.wrapping_sub(info.dwTime) as u64)
    }

    /// Resets the system idle timer with input no application reacts to: the pointer moves one
    /// pixel and straight back, and F15 (absent from real keyboards) is released without a press.
    fn nudge() {
        let mouse = |dx| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT { dx, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0 },
            },
        };
        let key_up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VK_F15, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 },
            },
        };
        let inputs = [mouse(1), mouse(-1), key_up];
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
    }
}

#[cfg(windows)]
fn main() {
    app::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("green-state only runs on Windows");
}
