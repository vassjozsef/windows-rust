use std::mem;
use windows::{
    core::{HSTRING, PWSTR},
    Foundation::Collections::StringMap,
    Graphics::Capture::GraphicsCaptureItem,
    Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM},
    Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1},
    Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
    },
    Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
    Win32::Graphics::Dxgi::IDXGIDevice,
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
    Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    Win32::System::WinRT::{RoActivateInstance, RoGetActivationFactory},
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetAncestor, GetShellWindow, GetWindowLongA, GetWindowTextW, IsWindowVisible,
        GA_ROOT, GWL_STYLE, WINDOW_STYLE, WS_DISABLED,
    },
};

fn main() -> windows::core::Result<()> {
    // StringMap test
    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };
    let instance = unsafe { RoActivateInstance("Windows.Foundation.Collections.StringMap") }?;
    dbg!(&instance);
    let map = windows::core::Interface::cast::<StringMap>(&instance)?;
    map.Insert("Hello", "World")?;
    dbg!(&map);
    println!("Map size: {}", map.Size()?);

    // Window enumeration to get HWND of window to be captured
    let mut window: HWND = HWND::default();
    let ptr = &mut window as *mut HWND;
    let par = LPARAM(ptr as isize);
    unsafe { EnumWindows(Some(enum_window), par) };
    dbg!(window);

    // Create GrpahicsCaptureItem
    let class_name: HSTRING = HSTRING::from("Windows.Graphics.Capture.GraphicsCaptureItem");
    let interop =
        unsafe { RoGetActivationFactory::<HSTRING, IGraphicsCaptureItemInterop>(class_name) }?;
    dbg!(&interop);
    let item = unsafe { interop.CreateForWindow::<HWND, GraphicsCaptureItem>(window) }?;
    let name = item.DisplayName()?;
    println!("Window to be capture: {}", name);

    // Create IDirectD3Device
    let d3_device = create_d3d_device().ok().unwrap();
    dbg!(&d3_device);
    let dxgi_device_ptr = &d3_device as *const _ as *const IDXGIDevice;
    let dxgi_device = unsafe {&*dxgi_device_ptr};
    dbg!(dxgi_device);
    let direct3d_device = unsafe { CreateDirect3D11DeviceFromDXGIDevice(dxgi_device) }?;
    dbg!(direct3d_device);

    Ok(())
}

fn create_d3d_device() -> windows::core::Result<ID3D11Device> {
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    let device_type = D3D_DRIVER_TYPE_HARDWARE;
    let mut device = None;
    let levels = &[D3D_FEATURE_LEVEL_11_1];

    unsafe {
        D3D11CreateDevice(
            None,
            device_type,
            HINSTANCE::default(),
            flags,
            levels.as_ptr(),
            levels.len() as u32,
            D3D11_SDK_VERSION,
            &mut device,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
        .map(|()| device.unwrap())
    }
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
        let result =
            DwmGetWindowAttribute(window, DWMWA_CLOAKED, ptr, mem::size_of::<i32>() as u32);
        if result.is_ok() && cloaked as u32 == DWM_CLOAKED_SHELL {
            return true.into();
        }
    }

    unsafe {
        let mut text: [u16; 512] = [0; 512];
        let ptr: PWSTR = PWSTR(&mut text as *mut u16);
        let len = GetWindowTextW(window, ptr, text.len() as i32);
        if len == 0 {
            return true.into();
        }
        let text = String::from_utf16_lossy(&text[..len as usize]);

        print!("{:?}: {}\n", window, text);

        let ptr = out.0 as *mut HWND;
        *ptr = window;

        // stop after first window
        false.into()
    }
}
