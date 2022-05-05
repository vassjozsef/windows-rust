use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use windows::{
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    Win32::UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, MSG},
};

use crate::capturer::Capturer;
use crate::window_enum::Windows;

mod capturer;
mod window_enum;

fn main() -> windows::core::Result<()> {
    println!("Thread: {:?}", std::thread::current().id());
    // Window enumeration to get HWND of window to be captured
    let windows = Windows::enumerate();
    dbg!(&windows);

    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };

    let should_quit = Arc::new(AtomicBool::new(false));
    let c_should_quit = should_quit.clone();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        // must be created on the same thread as the message loop
        let capturer = Capturer::new(windows[0].hwnd).unwrap();
        capturer.start().ok();
        let mut message = MSG::default();
        while !c_should_quit.load(Ordering::Acquire) {
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
        if id % 100 == 0 {
            println!(
                "Thread: {:?}, frame: {:?}",
                std::thread::current().id(),
                frame
            );
        }

        if id >= 500 {
            should_quit.store(true, Ordering::SeqCst);
        }
    }

    handle.join().unwrap();

    Ok(())
}
