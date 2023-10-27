use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

use crate::capturer::Capturer;
use crate::screen_enum::Screens;
use crate::window_enum::Windows;

mod capturer;
mod screen_enum;
mod sys;
mod window_enum;

fn main() -> windows::core::Result<()> {
    println!(
        "Main thread: {:?}, waiting for 10 seconds",
        std::thread::current().id()
    );
    thread::sleep(Duration::from_secs(10));

    // Window enumeration to get HWND of window to be captured
    let windows = Windows::enumerate();
    dbg!(&windows);

    // Screen enumeration
    let screens = Screens::enumerate();
    dbg!(&screens);

    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };

    let should_quit = Arc::new(AtomicBool::new(false));
    let c_should_quit = should_quit.clone();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        println!(
            "Creating capturer on thread: {:?}",
            std::thread::current().id()
        );

        /*
                // select window
                let index = windows
                    .iter()
                    .position(|w| w.name.starts_with("notes"))
                    .unwrap_or_default();
                let capturer = Capturer::new(Some(windows[index].hwnd), None).unwrap();
        */
        let capturer = Capturer::new(None, Some(screens[0].hmonitor)).unwrap();
        capturer.start().ok();
        while !c_should_quit.load(Ordering::Acquire) {
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
