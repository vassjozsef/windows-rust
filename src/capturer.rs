use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use windows::{
    core::{IInspectable, Interface, HSTRING},
    Foundation::{EventRegistrationToken, TypedEventHandler},
    Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Graphics::{Capture::GraphicsCaptureSession, DirectX::DirectXPixelFormat},
    System::DispatcherQueueController,
    Win32::Foundation::{HINSTANCE, HWND},
    Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1},
    Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
    },
    Win32::Graphics::Dxgi::IDXGIDevice,
    Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
    Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    Win32::System::WinRT::{
        CreateDispatcherQueueController, DispatcherQueueOptions, RoGetActivationFactory,
        DQTAT_COM_STA, DQTYPE_THREAD_CURRENT,
    },
};

fn create_dispatcher_queu_controller() -> windows::core::Result<DispatcherQueueController> {
    let options = DispatcherQueueOptions {
        dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
        threadType: DQTYPE_THREAD_CURRENT,
        apartmentType: DQTAT_COM_STA,
    };

    unsafe { CreateDispatcherQueueController(options) }
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
            levels,
            D3D11_SDK_VERSION,
            &mut device,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
        .map(|()| device.unwrap())
    }
}

pub struct Capturer {
    _hwnd: HWND,

    frame_pool: Direct3D11CaptureFramePool,

    session: GraphicsCaptureSession,

    frame_arrived: EventRegistrationToken,

    // not sure it is needed, we just keeping it here for now
    _controller: DispatcherQueueController,

    pub frame_count: Arc<AtomicU32>,
}

impl Capturer {
    pub fn new(hwnd: HWND) -> windows::core::Result<Capturer> {
        // pump
        let controller = create_dispatcher_queu_controller()?;

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
        let interop =
            unsafe { RoGetActivationFactory::<HSTRING, IGraphicsCaptureItemInterop>(class_name) }?;
        dbg!(&interop);
        let item = unsafe { interop.CreateForWindow::<HWND, GraphicsCaptureItem>(hwnd) }?;
        let name = item.DisplayName()?;
        let mut dim = item.Size()?;
        println!("Window to be capture: {}, dimensions: {:?}", name, dim);

        // Start capture
        let frame_pool = Direct3D11CaptureFramePool::Create(
            device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            dim,
        )?;
        let session = frame_pool.CreateCaptureSession(item)?;

        let frame_count = Arc::new(AtomicU32::new(0));
        let frame_count_handler = frame_count.clone();
        let frame_pool_handler = frame_pool.clone();
        type Handler = TypedEventHandler<Direct3D11CaptureFramePool, IInspectable>;
        let handler = Handler::new(move |sender, _| {
            let count = frame_count_handler.fetch_add(1, Ordering::Acquire);

            let sender = sender.as_ref().unwrap();
            let frame = sender.TryGetNextFrame()?;
            let size = frame.ContentSize()?;

            if count % 10 == 0 {
                println!(
                    "Thread: {:?}, frames captured: {}, last size: {:?}",
                    std::thread::current().id(),
                    count,
                    size
                );
            }

            if dim != size {
                dim = size;

                println!("Frame size changed to {:?}", dim);
                // device still has threading issues, but FramePool is fine
                // frame_pool_handler.as_ref().Recreate(device_wrapper_handler.as_ref().device,  DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, dim);
                frame_pool_handler.DispatcherQueue()?;
            }
            Ok(())
        });

        let frame_arrived = frame_pool.FrameArrived(handler)?;

        Ok(Capturer {
            _hwnd: hwnd,
            frame_pool: frame_pool,
            session: session,
            frame_arrived: frame_arrived,
            _controller: controller,
            frame_count: frame_count,
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
