use crate::sys::{create_direct3d11_device_from_dxgi_device, ro_get_activation_factory};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use windows::{
    core::{Abi, Error, IInspectable, Interface, HRESULT, HSTRING},
    Foundation::{EventRegistrationToken, TypedEventHandler},
    Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Graphics::{Capture::GraphicsCaptureSession, DirectX::DirectXPixelFormat},
    Win32::Foundation::{HINSTANCE, HWND},
    Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1},
    Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11Texture2D, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_SDK_VERSION,
    },
    Win32::Graphics::Dxgi::IDXGIDevice,
    Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
};

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
            levels,
            D3D11_SDK_VERSION,
            &mut device,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
        .map(|()| device.unwrap())
    }
}

#[allow(non_snake_case)]
pub unsafe fn CreateDirect3D11DeviceFromDXGIDevice<
    'a,
    Param0: ::windows::core::IntoParam<'a, IDXGIDevice>,
>(
    dxgidevice: Param0,
) -> ::windows::core::Result<::windows::core::IInspectable> {
    {
        let mut result__: *mut ::core::ffi::c_void = ::core::mem::zeroed();
        //    CreateDirect3D11DeviceFromDXGIDevice(dxgidevice.into_param().abi(), ::core::mem::transmute(&mut result__)).from_abi::<::windows::core::IInspectable>(result__)
        create_direct3d11_device_from_dxgi_device(
            dxgidevice.into_param().abi(),
            ::core::mem::transmute(&mut result__),
        )
        .from_abi::<::windows::core::IInspectable>(result__)
    }
}

#[derive(Debug)]
pub struct Frame {
    pub ts: Instant,
    pub id: u32,
}

impl Frame {
    pub fn new(ts: Instant, id: u32) -> Self {
        Frame { ts: ts, id: id }
    }
}

pub struct IDirect3DDeviceWrapper {
    pub device: IDirect3DDevice,
}

impl IDirect3DDeviceWrapper {
    pub fn new(device: IDirect3DDevice) -> IDirect3DDeviceWrapper {
        IDirect3DDeviceWrapper { device: device }
    }
}

unsafe impl Send for IDirect3DDeviceWrapper {}

pub struct Capturer {
    _hwnd: HWND,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    frame_arrived: EventRegistrationToken,
    pub frame_count: Arc<AtomicU32>,
    pub frame: Arc<Mutex<Option<Frame>>>,
}

impl Capturer {
    pub fn new(hwnd: HWND) -> windows::core::Result<Capturer> {
        // Create IDirectD3Device
        let d3d_device = create_d3d_device().ok().unwrap();
        dbg!(&d3d_device);
        let dxgi_device = d3d_device.cast::<IDXGIDevice>()?;
        dbg!(&dxgi_device);
        let direct3d_device = unsafe { CreateDirect3D11DeviceFromDXGIDevice(dxgi_device) }?;
        dbg!(&direct3d_device);
        let device = direct3d_device.cast::<IDirect3DDevice>()?;
        dbg!(&device);

        // Create GrpahicsCaptureItem
        let class_name: HSTRING = HSTRING::from("Windows.Graphics.Capture.GraphicsCaptureItem");
        let mut factory = std::ptr::null_mut();
        let hr = unsafe {
            ro_get_activation_factory(class_name, IGraphicsCaptureItemInterop::IID, &mut factory)
        };
        if hr != HRESULT(0) {
            return Err(Error::from(hr));
        }
        let interop = unsafe { IGraphicsCaptureItemInterop::from_abi(factory as *mut _)? };
        dbg!(&interop);
        let item = unsafe { interop.CreateForWindow::<HWND, GraphicsCaptureItem>(hwnd) }?;
        let name = item.DisplayName()?;
        let mut dim = item.Size()?;
        println!("Window to be capture: {}, dimensions: {:?}", name, dim);

        // Start capture
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            dim,
        )?;
        let session = frame_pool.CreateCaptureSession(item)?;

        let frame_count = Arc::new(AtomicU32::new(0));
        let c_frame_count = frame_count.clone();
        let c_frame_pool = frame_pool.clone();
        let frame = Arc::new(Mutex::new(None));
        let c_frame = frame.clone();
        let device_wrapper = IDirect3DDeviceWrapper::new(device);
        type Handler = TypedEventHandler<Direct3D11CaptureFramePool, IInspectable>;
        let handler = Handler::new(move |sender, _| {
            let count = c_frame_count.fetch_add(1, Ordering::Acquire);

            let sender = sender.as_ref().unwrap();
            let captured_frame = sender.TryGetNextFrame()?;
            let size = captured_frame.ContentSize()?;
            let surface = captured_frame.Surface().ok().unwrap();
            let access = surface.cast::<IDirect3DDxgiInterfaceAccess>()?;
            let texture = unsafe { access.GetInterface::<ID3D11Texture2D>()? };

            c_frame
                .lock()
                .unwrap()
                .replace(Frame::new(Instant::now(), count));

            if count % 10 == 0 {
                println!(
                    "Thread: {:?}, frames captured: {}, last size: {:?}, texture: {:?}",
                    std::thread::current().id(),
                    count,
                    size,
                    texture
                );
            }

            if dim != size {
                println!("Frame size changed to {:?}", size);

                dim = size;

                c_frame_pool.Recreate(
                    &device_wrapper.device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    dim,
                )?;
            }
            Ok(())
        });

        let frame_arrived = frame_pool.FrameArrived(handler)?;

        Ok(Capturer {
            _hwnd: hwnd,
            frame_pool: frame_pool,
            session: session,
            frame_arrived: frame_arrived,
            frame_count: frame_count,
            frame: frame,
        })
    }

    pub fn start(&self) -> windows::core::Result<()> {
        self.session.StartCapture()
    }

    pub fn stop(&self) -> windows::core::Result<()> {
        self.frame_pool.RemoveFrameArrived(self.frame_arrived)?;
        self.frame_pool.Close()?;
        self.session.Close()?;
        Ok(())
    }
}
