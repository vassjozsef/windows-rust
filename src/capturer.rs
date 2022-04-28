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
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
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
    frame_pool: Direct3D11CaptureFramePool,

    session: GraphicsCaptureSession,

    frame_arrived: EventRegistrationToken,

    _controller: DispatcherQueueController,
}

impl Capturer {
    pub fn new(window: HWND) -> windows::core::Result<Capturer> {
        unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED)? };

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
        let item = unsafe { interop.CreateForWindow::<HWND, GraphicsCaptureItem>(window) }?;
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

        let mut frame_count = 0;

        type Handler = TypedEventHandler<Direct3D11CaptureFramePool, IInspectable>;
        let handler = Handler::new(move |sender, _| {
            frame_count = frame_count + 1;

            let sender = sender.as_ref().unwrap();
            let frame = sender.TryGetNextFrame()?;
            let size = frame.ContentSize()?;

            if frame_count % 10 == 0 {
                println!("Frames captured: {}, last size: {:?}", frame_count, size);
            }

            if dim != size {
                dim = size;

                println!("Frame size changed to {:?}", dim);
                // some error of passing pointer between threads
                // frame_pool.Recreate(device,  DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, dim);
            }
            Ok(())
        });

        let frame_arrived = frame_pool.FrameArrived(handler)?;

        Ok(Capturer {
            frame_pool: frame_pool,
            session: session,
            frame_arrived: frame_arrived,
            _controller: controller,
        })
    }

    pub fn stop(&self) -> windows::core::Result<()> {
        self.frame_pool.RemoveFrameArrived(self.frame_arrived)?;
        self.frame_pool.Close()?;
        self.session.Close()?;
        Ok(())
    }

    pub fn start(&self) -> windows::core::Result<()> {
        self.session.StartCapture()
    }
}
