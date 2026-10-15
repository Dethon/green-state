#![windows_subsystem = "windows"]

#[cfg(windows)]
mod icon;
#[cfg(windows)]
mod registry;

#[cfg(windows)]
mod app {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use windows_sys::Win32::Foundation::{
        ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, SYSTEMTIME, WPARAM,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };
    use windows_sys::Win32::System::RemoteDesktop::{NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification};
    use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetTickCount};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetLastInputInfo, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        LASTINPUTINFO, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput, VK_F15,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics, MSG, RegisterClassW,
        SM_CXSMICON, SetTimer, TranslateMessage, WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK,
        WTS_SESSION_UNLOCK,
    };
    use windows_sys::w;

    use crate::{icon, registry};

    /// Teams flips to "Away" after five idle minutes; stay far below that.
    const IDLE_LIMIT: Duration = Duration::from_secs(60);
    const POLL_INTERVAL: Duration = Duration::from_secs(15);
    const DAY: u64 = 24 * 60 * 60;

    /// Pause lengths in seconds; `None` pauses until midnight.
    const PAUSES: [(&str, Option<u64>); 5] = [
        ("15 minutes", Some(15 * 60)),
        ("30 minutes", Some(30 * 60)),
        ("1 hour", Some(60 * 60)),
        ("2 hours", Some(2 * 60 * 60)),
        ("Rest of today", None),
    ];

    /// Monday to Friday, from the first hour up to (not including) the second; `None` is any time.
    const WORK_HOURS: [(&str, Option<(u16, u16)>); 4] = [
        ("Any time", None),
        ("Mon–Fri 8:00–17:00", Some((8, 17))),
        ("Mon–Fri 9:00–18:00", Some((9, 18))),
        ("Mon–Fri 8:00–18:00", Some((8, 18))),
    ];

    static ENABLED: AtomicBool = AtomicBool::new(true);
    static LOCKED: AtomicBool = AtomicBool::new(false);
    /// Unix time in seconds at which a pause ends; anything in the past means not paused.
    static PAUSED_UNTIL: AtomicU64 = AtomicU64::new(0);
    static WORK_HOURS_PRESET: AtomicUsize = AtomicUsize::new(0);

    #[derive(Clone, Copy, PartialEq)]
    enum Status {
        Off,
        Paused { until: u64 },
        OffHours,
        Locked,
        Active,
    }

    pub fn run() {
        unsafe {
            CreateMutexW(null(), 0, w!("Local\\green-state-single-instance"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        ENABLED.store(registry::get_setting(w!("Enabled")).is_none_or(|value| value != 0), Ordering::Relaxed);
        let preset = registry::get_setting(w!("WorkHours")).map_or(0, |value| value as usize);
        WORK_HOURS_PRESET.store(preset.min(WORK_HOURS.len() - 1), Ordering::Relaxed);
        let autostart = registry::starts_with_windows();
        if autostart {
            // Rewrite the entry so it follows the executable if it was moved.
            registry::set_start_with_windows(true);
        }

        let toggle = CheckMenuItem::new("Keep Teams green", true, ENABLED.load(Ordering::Relaxed), None);
        let pauses: Vec<MenuItem> = PAUSES.iter().map(|(label, _)| MenuItem::new(label, true, None)).collect();
        let resume = MenuItem::new("Resume", false, None);
        let pause_menu = Submenu::new("Pause", true);
        for item in &pauses {
            pause_menu.append(item).expect("failed to build tray menu");
        }
        pause_menu.append_items(&[&PredefinedMenuItem::separator(), &resume]).expect("failed to build tray menu");
        let presets: Vec<CheckMenuItem> = WORK_HOURS
            .iter()
            .enumerate()
            .map(|(index, (label, _))| {
                CheckMenuItem::new(label, true, index == WORK_HOURS_PRESET.load(Ordering::Relaxed), None)
            })
            .collect();
        let hours_menu = Submenu::new("Work hours", true);
        for item in &presets {
            hours_menu.append(item).expect("failed to build tray menu");
        }
        let startup = CheckMenuItem::new("Start with Windows", true, autostart, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &toggle,
            &pause_menu,
            &hours_menu,
            &PredefinedMenuItem::separator(),
            &startup,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .expect("failed to build tray menu");

        // Dropping the tray handle removes the icon, so it lives until `run` returns.
        let mut shown = status();
        let mut shown_tooltip = tooltip(shown);
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(&shown_tooltip)
            .with_icon(tray_icon(shown == Status::Active))
            .build()
            .expect("failed to create tray icon");

        watch_session_lock();
        thread::spawn(keep_active);

        unsafe {
            // Wakes the loop every second so pauses and work hours show up in the tray when they end.
            SetTimer(null_mut(), 0, 1000, None);

            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // Tray and menu events are emitted from the tray's window procedure, i.e. during dispatch.
                while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } =
                        event
                    {
                        toggle.set_checked(!toggle.is_checked());
                        set_enabled(toggle.is_checked());
                    }
                }
                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == *quit.id() {
                        return;
                    }
                    if event.id == *toggle.id() {
                        set_enabled(toggle.is_checked());
                    }
                    if event.id == *startup.id() {
                        registry::set_start_with_windows(startup.is_checked());
                    }
                    if event.id == *resume.id() {
                        PAUSED_UNTIL.store(0, Ordering::Relaxed);
                    }
                    if let Some(index) = pauses.iter().position(|item| event.id == *item.id()) {
                        let length = PAUSES[index].1.unwrap_or_else(|| DAY - seconds_into_day());
                        PAUSED_UNTIL.store(unix_now() + length, Ordering::Relaxed);
                    }
                    if let Some(index) = presets.iter().position(|item| event.id == *item.id()) {
                        // Check items toggle themselves when clicked; make the presets behave like radio buttons.
                        for (other, item) in presets.iter().enumerate() {
                            item.set_checked(other == index);
                        }
                        WORK_HOURS_PRESET.store(index, Ordering::Relaxed);
                        registry::set_setting(w!("WorkHours"), index as u32);
                    }
                }

                let current = status();
                let current_tooltip = tooltip(current);
                if current_tooltip != shown_tooltip {
                    if (current == Status::Active) != (shown == Status::Active) {
                        let _ = tray.set_icon(Some(tray_icon(current == Status::Active)));
                    }
                    let _ = tray.set_tooltip(Some(&current_tooltip));
                    resume.set_enabled(matches!(current, Status::Paused { .. }));
                    shown = current;
                    shown_tooltip = current_tooltip;
                }
            }
        }
    }

    /// Switching on also ends any pause, so the toggle always does what it says.
    fn set_enabled(enabled: bool) {
        ENABLED.store(enabled, Ordering::Relaxed);
        if enabled {
            PAUSED_UNTIL.store(0, Ordering::Relaxed);
        }
        registry::set_setting(w!("Enabled"), enabled as u32);
    }

    fn status() -> Status {
        if !ENABLED.load(Ordering::Relaxed) {
            return Status::Off;
        }
        let until = PAUSED_UNTIL.load(Ordering::Relaxed);
        if unix_now() < until {
            return Status::Paused { until };
        }
        if let Some((start, end)) = WORK_HOURS[WORK_HOURS_PRESET.load(Ordering::Relaxed)].1 {
            let now = local_time();
            let weekday = (1..=5).contains(&now.wDayOfWeek); // 0 is Sunday
            if !weekday || !(start..end).contains(&now.wHour) {
                return Status::OffHours;
            }
        }
        if LOCKED.load(Ordering::Relaxed) {
            return Status::Locked;
        }
        Status::Active
    }

    fn tooltip(status: Status) -> String {
        match status {
            Status::Off => "Green State: off".into(),
            Status::Paused { until } => {
                let end = seconds_into_day() + until.saturating_sub(unix_now());
                if end >= DAY {
                    "Green State: paused for the rest of the day".into()
                } else {
                    format!("Green State: paused until {:02}:{:02}", end / 3600, end / 60 % 60)
                }
            }
            Status::OffHours => "Green State: outside work hours".into(),
            Status::Locked => "Green State: standing by while locked".into(),
            Status::Active => "Green State: keeping Teams green".into(),
        }
    }

    fn tray_icon(active: bool) -> Icon {
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
        Icon::from_rgba(icon::render(size, active), size, size).expect("invalid icon buffer")
    }

    fn unix_now() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
    }

    fn local_time() -> SYSTEMTIME {
        unsafe {
            let mut time = zeroed();
            GetLocalTime(&mut time);
            time
        }
    }

    fn seconds_into_day() -> u64 {
        let now = local_time();
        now.wHour as u64 * 3600 + now.wMinute as u64 * 60 + now.wSecond as u64
    }

    /// Lock and unlock notifications are delivered to a window, so create one that is never shown.
    fn watch_session_lock() {
        unsafe {
            let instance = GetModuleHandleW(null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(session_window_proc),
                hInstance: instance,
                lpszClassName: w!("green-state-session"),
                ..zeroed()
            };
            RegisterClassW(&class);
            let window =
                CreateWindowExW(0, class.lpszClassName, null(), 0, 0, 0, 0, 0, null_mut(), null_mut(), instance, null());
            WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION);
        }
    }

    unsafe extern "system" fn session_window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if message == WM_WTSSESSION_CHANGE {
            match wparam as u32 {
                WTS_SESSION_LOCK => LOCKED.store(true, Ordering::Relaxed),
                WTS_SESSION_UNLOCK => LOCKED.store(false, Ordering::Relaxed),
                _ => {}
            }
        }
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    /// Blocks sleep and display-off, and fakes input whenever the user has been idle too long.
    /// Real activity is never interfered with: nothing is injected while the user is at the keyboard.
    fn keep_active() {
        loop {
            // The execution state is per thread, so it has to be (re)set from this one.
            let state = match status() {
                Status::Active => {
                    if idle_time() >= IDLE_LIMIT {
                        nudge();
                    }
                    ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
                }
                // Teams shows "Away" on a locked session whatever happens and Windows rejects injected
                // input there, so let the screen sleep but keep the machine (and Teams) connected.
                Status::Locked => ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
                Status::Off | Status::Paused { .. } | Status::OffHours => ES_CONTINUOUS,
            };
            unsafe { SetThreadExecutionState(state) };
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
