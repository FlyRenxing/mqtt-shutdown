//! WinRT AppWindow helpers. windows-reactor does not expose Closing, which is
//! the official way to cancel the WinUI 3 close button.

use std::ffi::c_void;

use windows_core::{
    self, IInspectable_Vtbl, IUnknown, IUnknown_Vtbl, Interface, Ref, Result, RuntimeName,
    RuntimeType,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct WindowId {
    value: u64,
}

windows_core::imp::define_interface!(
    IAppWindow,
    IAppWindow_Vtbl,
    0xcfa788b3_643b_5c5e_ad4e_321d48a82acd
);
impl RuntimeType for IAppWindow {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}

#[repr(C)]
pub struct IAppWindow_Vtbl {
    base__: IInspectable_Vtbl,
    id: usize,
    is_shown_in_switchers: usize,
    set_is_shown_in_switchers: unsafe extern "system" fn(*mut c_void, bool) -> windows_core::HRESULT,
    is_visible: usize,
    owner_window_id: usize,
    position: usize,
    presenter: usize,
    size: usize,
    title: usize,
    set_title: usize,
    title_bar: usize,
    destroy: usize,
    hide: unsafe extern "system" fn(*mut c_void) -> windows_core::HRESULT,
    r#move: usize,
    move_and_resize: usize,
    move_and_resize_relative: usize,
    resize: usize,
    set_icon: usize,
    set_icon_with_id: usize,
    set_presenter: usize,
    set_presenter_by_kind: usize,
    show: unsafe extern "system" fn(*mut c_void) -> windows_core::HRESULT,
    show_with_activation: unsafe extern "system" fn(*mut c_void, bool) -> windows_core::HRESULT,
    changed: usize,
    remove_changed: usize,
    closing: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i64) -> windows_core::HRESULT,
    remove_closing: unsafe extern "system" fn(*mut c_void, i64) -> windows_core::HRESULT,
    destroying: usize,
    remove_destroying: usize,
}

impl IAppWindow {
    fn set_shown_in_switchers(&self, value: bool) -> Result<()> {
        unsafe {
            (Interface::vtable(self).set_is_shown_in_switchers)(Interface::as_raw(self), value).ok()
        }
    }

    fn hide(&self) -> Result<()> {
        unsafe { (Interface::vtable(self).hide)(Interface::as_raw(self)).ok() }
    }

    fn show(&self, activate: bool) -> Result<()> {
        unsafe {
            (Interface::vtable(self).show_with_activation)(Interface::as_raw(self), activate).ok()
        }
    }

    fn closing<F>(&self, handler: F) -> Result<i64>
    where
        F: Fn(Ref<AppWindow>, Ref<AppWindowClosingEventArgs>) + 'static,
    {
        let handler = TypedEventHandler::<AppWindow, AppWindowClosingEventArgs>::new(handler);
        unsafe {
            let mut token = 0i64;
            (Interface::vtable(self).closing)(
                Interface::as_raw(self),
                Interface::as_raw(&handler),
                &mut token,
            )
            .map(|| token)
        }
    }
}

windows_core::imp::define_interface!(
    IAppWindowStatics,
    IAppWindowStatics_Vtbl,
    0x3c315c24_d540_5d72_b518_b226b83627cb
);

#[repr(C)]
pub struct IAppWindowStatics_Vtbl {
    base__: IInspectable_Vtbl,
    create: usize,
    create_with_presenter: usize,
    create_with_presenter_and_owner: usize,
    get_from_window_id:
        unsafe extern "system" fn(*mut c_void, WindowId, *mut *mut c_void) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(
    IAppWindowClosingEventArgs,
    IAppWindowClosingEventArgs_Vtbl,
    0x0e09d90b_2261_590b_9ad1_8504991d8754
);
impl RuntimeType for IAppWindowClosingEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}

#[repr(C)]
pub struct IAppWindowClosingEventArgs_Vtbl {
    base__: IInspectable_Vtbl,
    cancel: unsafe extern "system" fn(*mut c_void, *mut bool) -> windows_core::HRESULT,
    set_cancel: unsafe extern "system" fn(*mut c_void, bool) -> windows_core::HRESULT,
}

impl IAppWindowClosingEventArgs {
    fn set_cancel(&self, value: bool) -> Result<()> {
        unsafe { (Interface::vtable(self).set_cancel)(Interface::as_raw(self), value).ok() }
    }
}

#[repr(transparent)]
#[derive(Clone)]
struct AppWindow(IUnknown);
windows_core::imp::interface_hierarchy!(AppWindow, IUnknown, windows_core::IInspectable);
impl RuntimeType for AppWindow {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IAppWindow>();
}
unsafe impl Interface for AppWindow {
    type Vtable = <IAppWindow as Interface>::Vtable;
    const IID: windows_core::GUID = <IAppWindow as Interface>::IID;
}
impl RuntimeName for AppWindow {
    const NAME: &'static str = "Microsoft.UI.Windowing.AppWindow";
}

#[repr(transparent)]
#[derive(Clone)]
struct AppWindowClosingEventArgs(IUnknown);
windows_core::imp::interface_hierarchy!(
    AppWindowClosingEventArgs,
    IUnknown,
    windows_core::IInspectable
);
impl RuntimeType for AppWindowClosingEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IAppWindowClosingEventArgs>();
}
unsafe impl Interface for AppWindowClosingEventArgs {
    type Vtable = <IAppWindowClosingEventArgs as Interface>::Vtable;
    const IID: windows_core::GUID = <IAppWindowClosingEventArgs as Interface>::IID;
}
impl core::ops::Deref for AppWindowClosingEventArgs {
    type Target = IAppWindowClosingEventArgs;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl RuntimeName for AppWindowClosingEventArgs {
    const NAME: &'static str = "Microsoft.UI.Windowing.AppWindowClosingEventArgs";
}

#[repr(transparent)]
#[derive(Clone)]
struct TypedEventHandler<TSender, TResult>(IUnknown, core::marker::PhantomData<(TSender, TResult)>)
where
    TSender: RuntimeType + 'static,
    TResult: RuntimeType + 'static;

unsafe impl<TSender: RuntimeType + 'static, TResult: RuntimeType + 'static> Interface
    for TypedEventHandler<TSender, TResult>
{
    type Vtable = TypedEventHandlerVtbl<TSender, TResult>;
    const IID: windows_core::GUID =
        windows_core::GUID::from_signature(<Self as RuntimeType>::SIGNATURE);
}

impl<TSender: RuntimeType + 'static, TResult: RuntimeType + 'static> RuntimeType
    for TypedEventHandler<TSender, TResult>
{
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({9de1c534-6ae1-11e0-84e1-18a905bcc53f}")
        .push_slice(b";")
        .push_other(TSender::SIGNATURE)
        .push_slice(b";")
        .push_other(TResult::SIGNATURE)
        .push_slice(b")");
}

impl<TSender: RuntimeType + 'static, TResult: RuntimeType + 'static>
    TypedEventHandler<TSender, TResult>
{
    fn new<F: Fn(Ref<TSender>, Ref<TResult>) + 'static>(invoke: F) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(
            &TypedEventHandlerBox::<TSender, TResult, F>::VTABLE,
            invoke,
        );
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }
}

#[repr(C)]
struct TypedEventHandlerVtbl<TSender, TResult>
where
    TSender: RuntimeType + 'static,
    TResult: RuntimeType + 'static,
{
    base__: IUnknown_Vtbl,
    invoke: unsafe extern "system" fn(
        this: *mut c_void,
        sender: windows_core::AbiType<TSender>,
        args: windows_core::AbiType<TResult>,
    ) -> windows_core::HRESULT,
    _sender: core::marker::PhantomData<TSender>,
    _result: core::marker::PhantomData<TResult>,
}

struct TypedEventHandlerBox<TSender, TResult, F>(core::marker::PhantomData<(TSender, TResult, F)>)
where
    TSender: RuntimeType + 'static,
    TResult: RuntimeType + 'static,
    F: Fn(Ref<TSender>, Ref<TResult>) + 'static;

impl<TSender, TResult, F> TypedEventHandlerBox<TSender, TResult, F>
where
    TSender: RuntimeType + 'static,
    TResult: RuntimeType + 'static,
    F: Fn(Ref<TSender>, Ref<TResult>) + 'static,
{
    const VTABLE: TypedEventHandlerVtbl<TSender, TResult> = TypedEventHandlerVtbl {
        base__: IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<
                TypedEventHandler<TSender, TResult>,
                F,
            >::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<TypedEventHandler<TSender, TResult>, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<TypedEventHandler<TSender, TResult>, F>::Release,
        },
        invoke: Self::invoke,
        _sender: core::marker::PhantomData,
        _result: core::marker::PhantomData,
    };

    unsafe extern "system" fn invoke(
        this: *mut c_void,
        sender: windows_core::AbiType<TSender>,
        args: windows_core::AbiType<TResult>,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut c_void
                as *mut windows_core::imp::DelegateBox<TypedEventHandler<TSender, TResult>, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&args));
            windows_core::HRESULT(0)
        }
    }
}

windows_core::imp::define_interface!(
    IWindow2,
    IWindow2_Vtbl,
    0x42febaa5_1c32_522a_a591_57618c6f665d
);

#[repr(C)]
pub struct IWindow2_Vtbl {
    base__: IInspectable_Vtbl,
    system_backdrop: usize,
    set_system_backdrop: usize,
    app_window: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> windows_core::HRESULT,
}

fn from_active_xaml_window() -> Option<IAppWindow> {
    windows_reactor::with_active_host(|host| unsafe {
        let window = host.window();
        let unknown = {
            let ptr = window as *const _ as *const IUnknown;
            (*ptr).clone()
        };
        let window2: IWindow2 = unknown.cast().ok()?;
        let mut raw = core::ptr::null_mut();
        (Interface::vtable(&window2).app_window)(Interface::as_raw(&window2), &mut raw)
            .ok()
            .ok()?;
        windows_core::Type::from_abi(raw).ok()
    })
    .flatten()
}

fn window_id_from_hwnd(hwnd: *mut c_void) -> Option<WindowId> {
    type FnGetId = unsafe extern "system" fn(*mut c_void, *mut WindowId) -> i32;
    unsafe {
        let lib_name: Vec<u16> = "Microsoft.Internal.FrameworkUdk.dll"
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect();
        let lib = windows_sys::Win32::System::LibraryLoader::LoadLibraryW(lib_name.as_ptr());
        if lib.is_null() {
            return Some(WindowId {
                value: hwnd as usize as u64,
            });
        }
        let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            lib,
            windows_sys::core::PCSTR::from(b"Windowing_GetWindowIdFromWindow\0".as_ptr()),
        )?;
        let func: FnGetId = core::mem::transmute(proc);
        let mut id = WindowId { value: 0 };
        if func(hwnd, &mut id) >= 0 {
            Some(id)
        } else {
            None
        }
    }
}

fn from_hwnd(hwnd: *mut c_void) -> Result<IAppWindow> {
    if let Some(app) = from_active_xaml_window() {
        return Ok(app);
    }
    let factory: IAppWindowStatics = windows_core::factory::<AppWindow, IAppWindowStatics>()?;
    let id = window_id_from_hwnd(hwnd).ok_or_else(|| {
        windows_core::Error::new(
            windows_core::HRESULT(0x80004005u32 as i32),
            "GetWindowIdFromWindow failed",
        )
    })?;
    unsafe {
        let mut result = core::ptr::null_mut();
        (Interface::vtable(&factory).get_from_window_id)(
            Interface::as_raw(&factory),
            id,
            &mut result,
        )
        .and_then(|| windows_core::Type::from_abi(result))
    }
}

pub struct AppWindowHandle {
    inner: IAppWindow,
}

unsafe impl Send for AppWindowHandle {}
unsafe impl Sync for AppWindowHandle {}

impl AppWindowHandle {
    pub fn from_hwnd(hwnd: *mut c_void) -> Option<Self> {
        from_hwnd(hwnd).ok().map(|inner| Self { inner })
    }

    pub fn hook_close<F>(&self, on_close: F) -> bool
    where
        F: Fn() -> bool + 'static,
    {
        self.inner
            .closing(move |_sender, args| {
                if !on_close()
                    && let Some(args) = args.as_ref()
                {
                    let _ = args.set_cancel(true);
                }
            })
            .is_ok()
    }

    pub fn hide(&self) {
        let _ = self.inner.set_shown_in_switchers(false);
        let _ = self.inner.hide();
    }

    pub fn show(&self) {
        let _ = self.inner.set_shown_in_switchers(true);
        let _ = self.inner.show(true);
    }
}
