use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetAncestor, GetMessageA, GetShellWindow, GetWindowLongA, GetWindowTextW,
        IsWindowVisible, GA_ROOT, GWL_STYLE, MSG, WINDOW_STYLE, WS_DISABLED,
    },
    Win32::{
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
        UI::WindowsAndMessaging::DispatchMessageA,
    },
};

use crate::capturer::Capturer;

mod capturer;

#[derive(Debug, Clone)]
pub struct Window {
    pub hwnd: HWND,
    pub name: String,
}

fn main() -> windows::core::Result<()> {
    println!("Thread: {:?}", std::thread::current().id());
    // Window enumeration to get HWND of window to be captured
    let mut windows: Vec<Window> = Vec::new();
    let param = LPARAM(&mut windows as *mut Vec<Window> as isize);
    unsafe { EnumWindows(Some(enum_window), param) };
    dbg!(&windows);

    let handle = std::thread::spawn(move || {
        // must be created on the same thread as the message loop
        let capturer = Capturer::new(windows[0].hwnd).ok().unwrap();
        capturer.start().ok();
        let duration = std::time::Duration::from_secs(5);
        let start = std::time::SystemTime::now();
        let mut message = MSG::default();
        while std::time::SystemTime::now() < start + duration {
            unsafe { GetMessageA(&mut message, None, 0, 0) };
            unsafe { DispatchMessageA(&message) };
        }
        capturer.stop().ok();
    });

    handle.join().unwrap();

    Ok(())
}

extern "system" fn enum_window(window: HWND, out: LPARAM) -> BOOL {
    let shell_window = unsafe { GetShellWindow() };
    if shell_window == window {
        return true.into();
    }

    unsafe {
        if !IsWindowVisible(window).as_bool() {
            return true.into();
        }
    }

    unsafe {
        if GetAncestor(window, GA_ROOT) != window {
            return true.into();
        }
    }

    unsafe {
        let style = GetWindowLongA(window, GWL_STYLE);
        let style = WINDOW_STYLE(style as u32);
        if style & WS_DISABLED == WS_DISABLED {
            return true.into();
        }
    }

    unsafe {
        let mut cloaked: i32 = 0;
        let ptr = &mut cloaked as *mut _ as *mut _;
        let result = DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            ptr,
            std::mem::size_of::<i32>() as u32,
        );
        if result.is_ok() && cloaked as u32 == DWM_CLOAKED_SHELL {
            return true.into();
        }
    }

    unsafe {
        let mut text: [u16; 512] = [0; 512];
        let len = GetWindowTextW(window, &mut text);
        if len == 0 {
            return true.into();
        }
        let text = String::from_utf16_lossy(&text[..len as usize]);

        println!(
            "Thread: {:?}, window: {:?}: {}",
            std::thread::current().id(),
            window,
            text
        );

        let windows = &mut *(out.0 as *mut Vec<Window>);
        windows.push(Window {
            hwnd: window,
            name: text,
        });
    }

    return true.into();
}
