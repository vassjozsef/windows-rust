use windows::{
    Win32::Foundation::{BOOL, LPARAM, RECT},
    Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
};

#[derive(Debug)]
pub struct Screen {
    pub hmonitor: HMONITOR,
}

#[derive(Debug)]
pub struct Screens;

impl Screens {
    pub fn enumerate() -> Vec<Screen> {
        let mut screens: Vec<Screen> = Vec::new();
        let param = LPARAM(&mut screens as *mut Vec<Screen> as isize);
        let hdc: HDC = HDC::default();
        unsafe { EnumDisplayMonitors(hdc, std::ptr::null(), Some(enum_screen), param) };
        screens
    }
}

extern "system" fn enum_screen(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    out: LPARAM,
) -> BOOL {
    unsafe {
        let screens = &mut *(out.0 as *mut Vec<Screen>);
        screens.push(Screen { hmonitor });
    }

    return true.into();
}
