use windows::{
    core::PWSTR,
    Foundation::Collections::StringMap,
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    Win32::System::WinRT::RoActivateInstance,
    Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW},
};

fn main() -> windows::core::Result<()> {
    unsafe { EnumWindows(Some(enum_window), LPARAM(0)) };

    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };

    let instance = unsafe { RoActivateInstance("Windows.Foundation.Collections.StringMap") }?;

    dbg!(&instance);
    let map = windows::core::Interface::cast::<StringMap>(&instance)?;
    map.Insert("Hello", "World")?;

    dbg!(&map);
    print!("Map size: {}", map.Size()?);

    Ok(())
}

extern "system" fn enum_window(window: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let mut text: [u16; 512] = [0; 512];
        let text_ptr2: PWSTR = PWSTR(&mut text as *mut u16);
        let len = GetWindowTextW(window, text_ptr2, text.len() as i32);
        let text = String::from_utf16_lossy(&text[..len as usize]);

        if !text.is_empty() {
            print!("{:?}: {}\n", window, text);
        }

        true.into()
    }
}
