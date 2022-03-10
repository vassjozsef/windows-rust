use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW}, 
    core::PWSTR,
};

fn main() -> windows::core::Result<()> {
    unsafe { EnumWindows(Some(enum_window), LPARAM(0)).ok() }
}

extern "system" fn enum_window(window: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let mut text: [u16; 512] = [0; 512];
        let text_ptr2: PWSTR = PWSTR(&mut text as *mut u16);
        let len = GetWindowTextW(window,text_ptr2, text.len() as i32);
        let text = String::from_utf16_lossy(&text[..len as usize]);

        if !text.is_empty() {
            print!("{:?}: {}\n", window, text);
        }

        true.into()
    }
}
