use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    Win32::UI::WindowsAndMessaging::{
        DispatchMessageA, EnumWindows, GetAncestor, GetMessageA, GetShellWindow, GetWindowLongA,
        GetWindowTextW, IsWindowVisible, GA_ROOT, GWL_STYLE, MSG, WINDOW_STYLE, WS_DISABLED,
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

    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };

    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_handle = should_quit.clone();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        // must be created on the same thread as the message loop
        let capturer = Capturer::new(windows[0].hwnd).unwrap();
        capturer.start().ok();
        let mut message = MSG::default();
        while !should_quit_handle.load(Ordering::Acquire) {
            unsafe { GetMessageA(&mut message, None, 0, 0) };
            unsafe { DispatchMessageA(&message) };
            if let Some(frame) = capturer.frame.lock().unwrap().take() {
                tx.send(frame).ok();
            }
        }
        let count = capturer.frame_count.load(Ordering::Acquire);
        println!("Frames captured: {}", count);
        capturer.stop().ok();
    });

    while let Some(frame) = rx.recv().ok() {
        let id = frame.id;
        if id % 30 == 0 {
            println!("Frame: {:?}", frame);
        }

        if id >= 500 {
            should_quit.store(true, Ordering::SeqCst);
        }
    }

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
