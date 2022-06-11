use ari::os::win::{Library, Symbol};
use lazy_static::lazy_static;
use std::ffi::c_void;
use windows::core::{GUID, HRESULT, HSTRING};

static WINRT_LIBRARY_NAME: &'static str = "api-ms-win-core-winrt-l1-1-0.dll";
static D3D11_LIBRARY_NANE: &'static str = "d3d11.dll";

static WINRT_ROGETACTIVATIONFACTORY_FUNCTION_NAME: &'static [u8; 23usize] =
    b"RoGetActivationFactory\0";
static CREATEDIRECT3D11DEVICEFROMDXGIDEVICE_FUNCTION_NAME: &'static [u8; 37usize] =
    b"CreateDirect3D11DeviceFromDXGIDevice\0";

pub type RoGetActivationFactoryFn = unsafe extern "system" fn(
    activatableClassId: HSTRING,
    iid: *const GUID,
    factory: *mut *mut c_void,
) -> HRESULT;

pub type CreateDirect3D11DeviceFromDXGIDeviceFn = unsafe extern "system" fn(
    dxgidevice: *mut core::ffi::c_void,
    graphicsdevice: *mut *mut ::core::ffi::c_void,
) -> HRESULT;

fn invoke<TFn>(function: TFn) -> HRESULT
where
    TFn: FnOnce() -> HRESULT,
{
    function()
}

lazy_static! {
    static ref FUN_RO_GET_ACTIVATION_FACTORY: Result<Symbol<RoGetActivationFactoryFn>, std::io::Error> = {
        let library = Library::open(WINRT_LIBRARY_NAME)?;
        let ro_get_activation_factory_fn: Symbol<RoGetActivationFactoryFn> =
            unsafe { library.find(WINRT_ROGETACTIVATIONFACTORY_FUNCTION_NAME) }?;
        Ok(ro_get_activation_factory_fn)
    };
    static ref FUN_CREATE_DIRECT_3D11_DEVICE_FROM_DXGI_DEVICE: Result<Symbol<CreateDirect3D11DeviceFromDXGIDeviceFn>, std::io::Error> = {
        let library = Library::open(D3D11_LIBRARY_NANE)?;
        let create_direct3d11_device_from_dxgi_device_fn: Symbol<
            CreateDirect3D11DeviceFromDXGIDeviceFn,
        > = unsafe { library.find(CREATEDIRECT3D11DEVICEFROMDXGIDEVICE_FUNCTION_NAME) }?;
        Ok(create_direct3d11_device_from_dxgi_device_fn)
    };
}

pub unsafe fn ro_get_activation_factory(
    activatable_class_id: HSTRING,
    iid: GUID,
    factory: *mut *mut c_void,
) -> HRESULT {
    let fun = FUN_RO_GET_ACTIVATION_FACTORY.as_ref().ok().unwrap();
    invoke(|| (fun)(activatable_class_id, &iid, factory))
}

pub unsafe fn create_direct3d11_device_from_dxgi_device(
    dxgidevice: *mut core::ffi::c_void,
    graphicsdevice: *mut *mut ::core::ffi::c_void,
) -> HRESULT {
    let fun = FUN_CREATE_DIRECT_3D11_DEVICE_FROM_DXGI_DEVICE
        .as_ref()
        .ok()
        .unwrap();
    invoke(|| (fun)(dxgidevice, graphicsdevice))
}
