#![allow(non_snake_case)]

mod intel_hex;

use std::ffi::c_void;
use std::fs;
use std::io::{self, BufReader, BufWriter};
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::{BSTR, GUID, HRESULT, Interface, PCWSTR, VARIANT};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE, E_POINTER, S_FALSE, S_OK, VARIANT_FALSE,
    VARIANT_TRUE,
};
use windows::Win32::System::Com::{
    DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO, IClassFactory, IClassFactory_Vtbl, IDispatch,
    IDispatch_Vtbl, ITypeInfo,
};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows::Win32::System::Ole::{LoadTypeLibEx, REGKIND_NONE};

const IID_IWINMERGESCRIPT: GUID = GUID::from_u128(0x9b4c9a71_62de_4a02_8d96_6cf7d65e7249);
const CLSID_WINMERGESCRIPT: GUID = GUID::from_u128(0xc6e98fb5_a2e4_4b83_9f5f_3f5b5105c4ae);

static GLOBAL_OBJECTS: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
struct WinMergeScript {
    vtbl: *const IWinMergeScript_Vtbl,
    ref_count: AtomicU32,
}

#[repr(C)]
struct ClassFactory {
    vtbl: *const IClassFactory_Vtbl,
    ref_count: AtomicU32,
}

#[repr(C)]
struct IWinMergeScript_Vtbl {
    pub base__: IDispatch_Vtbl,
    pub get_PluginEvent: unsafe extern "system" fn(*mut c_void, *mut BSTR) -> HRESULT,
    pub get_PluginDescription: unsafe extern "system" fn(*mut c_void, *mut BSTR) -> HRESULT,
    pub get_PluginFileFilters: unsafe extern "system" fn(*mut c_void, *mut BSTR) -> HRESULT,
    pub get_PluginIsAutomatic: unsafe extern "system" fn(*mut c_void, *mut i16) -> HRESULT,
    pub get_PluginExtendedProperties: unsafe extern "system" fn(*mut c_void, *mut BSTR) -> HRESULT,
    pub UnpackFile: unsafe extern "system" fn(*mut c_void, BSTR, BSTR, *mut i16, *mut i32, *mut i16) -> HRESULT,
    pub PackFile: unsafe extern "system" fn(*mut c_void, BSTR, BSTR, *mut i16, i32, *mut i16) -> HRESULT,
    pub ShowSettingsDialog: unsafe extern "system" fn(*mut c_void, *mut i16) -> HRESULT,
}

unsafe extern "system" fn wm_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    let iid = &*riid;
    if *iid == IDispatch::IID || *iid == windows::core::IUnknown::IID || *iid == IID_IWINMERGESCRIPT {
        *ppv = this;
        wm_add_ref(this);
        return S_OK;
    }

    E_NOINTERFACE
}

unsafe extern "system" fn wm_add_ref(this: *mut c_void) -> u32 {
    let obj = this.cast::<WinMergeScript>();
    (*obj).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn wm_release(this: *mut c_void) -> u32 {
    let obj = this.cast::<WinMergeScript>();
    let remaining = (*obj).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        GLOBAL_OBJECTS.fetch_sub(1, Ordering::Release);
        drop(Box::from_raw(obj));
    }
    remaining
}

unsafe extern "system" fn wm_get_type_info_count(
    _this: *mut c_void,
    pctinfo: *mut u32,
) -> HRESULT {
    if pctinfo.is_null() {
        return E_POINTER;
    }
    *pctinfo = 1;
    S_OK
}

unsafe extern "system" fn wm_get_type_info(
    _this: *mut c_void,
    _itinfo: u32,
    _lcid: u32,
    pptinfo: *mut *mut c_void,
) -> HRESULT {
    if pptinfo.is_null() {
        return E_POINTER;
    }
    match ensure_type_info() {
        Ok(ti) => {
            *pptinfo = ti.into_raw();
            S_OK
        }
        Err(err) => err,
    }
}

unsafe extern "system" fn wm_get_ids_of_names(
    _this: *mut c_void,
    _riid: *const GUID,
    rgsz_names: *const PCWSTR,
    c_names: u32,
    _lcid: u32,
    rg_dispid: *mut i32,
) -> HRESULT {
    match ensure_type_info() {
        Ok(ti) => match ti.GetIDsOfNames(rgsz_names, c_names, rg_dispid) {
            Ok(_) => S_OK,
            Err(e) => e.code(),
        },
        Err(err) => err,
    }
}

unsafe extern "system" fn wm_invoke(
    this: *mut c_void,
    dispid_member: i32,
    _riid: *const GUID,
    _lcid: u32,
    wflags: DISPATCH_FLAGS,
    pdispparams: *const DISPPARAMS,
    pvar_result: *mut MaybeUninit<VARIANT>,
    pexcepinfo: *mut EXCEPINFO,
    pu_arg_err: *mut u32,
) -> HRESULT {
    match ensure_type_info() {
        Ok(ti) => match ti.Invoke(
            this,
            dispid_member,
            wflags,
            pdispparams as *mut DISPPARAMS,
            pvar_result.cast::<VARIANT>(),
            pexcepinfo,
            pu_arg_err,
        ) {
            Ok(_) => S_OK,
            Err(e) => e.code(),
        },
        Err(err) => err,
    }
}

unsafe extern "system" fn wm_get_plugin_event(_this: *mut c_void, pval: *mut BSTR) -> HRESULT {
    if pval.is_null() {
        return E_POINTER;
    }
    *pval = BSTR::from("FILE_PACK_UNPACK");
    S_OK
}

unsafe extern "system" fn wm_get_plugin_description(_this: *mut c_void, pval: *mut BSTR) -> HRESULT {
    if pval.is_null() {
        return E_POINTER;
    }
    *pval = BSTR::from("Intel HEX expander (x64, unpack only)");
    S_OK
}

unsafe extern "system" fn wm_get_plugin_file_filters(_this: *mut c_void, pval: *mut BSTR) -> HRESULT {
    if pval.is_null() {
        return E_POINTER;
    }
    *pval = BSTR::from("\\.hex$;\\.ihx$");
    S_OK
}

unsafe extern "system" fn wm_get_plugin_is_automatic(_this: *mut c_void, pval: *mut i16) -> HRESULT {
    if pval.is_null() {
        return E_POINTER;
    }
    *pval = VARIANT_TRUE.0;
    S_OK
}

unsafe extern "system" fn wm_get_plugin_extended_properties(
    _this: *mut c_void,
    pval: *mut BSTR,
) -> HRESULT {
    if pval.is_null() {
        return E_POINTER;
    }
    *pval = BSTR::from("MenuCaption=Expand Intel HEX");
    S_OK
}

unsafe extern "system" fn wm_unpack_file(
    _this: *mut c_void,
    file_src: BSTR,
    file_dst: BSTR,
    pb_changed: *mut i16,
    p_subcode: *mut i32,
    pb_success: *mut i16,
) -> HRESULT {
    if !p_subcode.is_null() {
        *p_subcode = 0;
    }

    let src = file_src.to_string();
    let dst = file_dst.to_string();

    let success = expand_hex_file(&src, &dst).is_ok();

    if !pb_changed.is_null() {
        *pb_changed = if success { VARIANT_TRUE.0 } else { VARIANT_FALSE.0 };
    }
    if !pb_success.is_null() {
        *pb_success = if success { VARIANT_TRUE.0 } else { VARIANT_FALSE.0 };
    }

    S_OK
}

unsafe extern "system" fn wm_pack_file(
    _this: *mut c_void,
    _file_src: BSTR,
    _file_dst: BSTR,
    pb_changed: *mut i16,
    _p_subcode: i32,
    pb_success: *mut i16,
) -> HRESULT {
    if !pb_changed.is_null() {
        *pb_changed = VARIANT_FALSE.0;
    }
    if !pb_success.is_null() {
        *pb_success = VARIANT_FALSE.0;
    }
    S_OK
}

unsafe extern "system" fn wm_show_settings_dialog(_this: *mut c_void, pb_handled: *mut i16) -> HRESULT {
    if !pb_handled.is_null() {
        *pb_handled = VARIANT_FALSE.0;
    }
    E_FAIL
}

static WINMERGE_VTBL: IWinMergeScript_Vtbl = IWinMergeScript_Vtbl {
    base__: IDispatch_Vtbl {
        base__: windows::core::IUnknown_Vtbl {
            QueryInterface: wm_query_interface,
            AddRef: wm_add_ref,
            Release: wm_release,
        },
        GetTypeInfoCount: wm_get_type_info_count,
        GetTypeInfo: wm_get_type_info,
        GetIDsOfNames: wm_get_ids_of_names,
        Invoke: wm_invoke,
    },
    get_PluginEvent: wm_get_plugin_event,
    get_PluginDescription: wm_get_plugin_description,
    get_PluginFileFilters: wm_get_plugin_file_filters,
    get_PluginIsAutomatic: wm_get_plugin_is_automatic,
    get_PluginExtendedProperties: wm_get_plugin_extended_properties,
    UnpackFile: wm_unpack_file,
    PackFile: wm_pack_file,
    ShowSettingsDialog: wm_show_settings_dialog,
};

unsafe extern "system" fn cf_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    let iid = &*riid;
    if *iid == IClassFactory::IID || *iid == windows::core::IUnknown::IID {
        *ppv = this;
        cf_add_ref(this);
        return S_OK;
    }

    E_NOINTERFACE
}

unsafe extern "system" fn cf_add_ref(this: *mut c_void) -> u32 {
    let obj = this.cast::<ClassFactory>();
    (*obj).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn cf_release(this: *mut c_void) -> u32 {
    let obj = this.cast::<ClassFactory>();
    let remaining = (*obj).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        GLOBAL_OBJECTS.fetch_sub(1, Ordering::Release);
        drop(Box::from_raw(obj));
    }
    remaining
}

unsafe extern "system" fn cf_create_instance(
    _this: *mut c_void,
    p_unk_outer: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if !p_unk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }

    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    let obj = Box::new(WinMergeScript {
        vtbl: &WINMERGE_VTBL,
        ref_count: AtomicU32::new(1),
    });
    GLOBAL_OBJECTS.fetch_add(1, Ordering::Release);

    let ptr = Box::into_raw(obj) as *mut c_void;
    let result = wm_query_interface(ptr, riid, ppv);
    wm_release(ptr);
    result
}

unsafe extern "system" fn cf_lock_server(_this: *mut c_void, _f_lock: BOOL) -> HRESULT {
    S_OK
}

static CLASS_FACTORY_VTBL: IClassFactory_Vtbl = IClassFactory_Vtbl {
    base__: windows::core::IUnknown_Vtbl {
        QueryInterface: cf_query_interface,
        AddRef: cf_add_ref,
        Release: cf_release,
    },
    CreateInstance: cf_create_instance,
    LockServer: cf_lock_server,
};

fn ensure_type_info() -> Result<ITypeInfo, HRESULT> {
    unsafe { load_type_info() }
}

unsafe fn load_type_info() -> Result<ITypeInfo, HRESULT> {
    let mut module: windows::Win32::Foundation::HMODULE =
        windows::Win32::Foundation::HMODULE(std::ptr::null_mut());
    let addr = load_type_info as *const () as *const u16;
    if GetModuleHandleExW(
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        PCWSTR(addr),
        &mut module,
    )
    .is_err()
    {
        return Err(E_FAIL);
    }

    let mut path_buf = [0u16; 1024];
    let len = GetModuleFileNameW(module, &mut path_buf);
    if len == 0 {
        return Err(E_FAIL);
    }

    let path = windows::core::PCWSTR(path_buf.as_ptr());
    let typelib = LoadTypeLibEx(path, REGKIND_NONE).map_err(|e| e.code())?;
    let typeinfo = typelib.GetTypeInfoOfGuid(&IID_IWINMERGESCRIPT).map_err(|e| e.code())?;

    Ok(typeinfo)
}

fn expand_hex_file(src: &str, dst: &str) -> io::Result<()> {
    let input = BufReader::new(fs::File::open(src)?);
    let output = BufWriter::new(fs::File::create(dst)?);
    intel_hex::expand(input, output)
}

/// # Safety
///
/// `rclsid` and `riid` must point to valid `GUID` values. When non-null, `ppv`
/// must point to writable memory for one interface pointer.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    if *rclsid != CLSID_WINMERGESCRIPT {
        return E_NOINTERFACE;
    }

    let factory = Box::new(ClassFactory {
        vtbl: &CLASS_FACTORY_VTBL,
        ref_count: AtomicU32::new(1),
    });
    GLOBAL_OBJECTS.fetch_add(1, Ordering::Release);

    let ptr = Box::into_raw(factory) as *mut c_void;
    let result = cf_query_interface(ptr, riid, ppv);
    cf_release(ptr);
    result
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if GLOBAL_OBJECTS.load(Ordering::Acquire) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub extern "system" fn DllMain(_hinst: *mut c_void, _reason: u32, _reserved: *mut c_void) -> i32 {
    1
}

#[cfg(test)]
mod com_tests {
    use super::*;

    #[test]
    fn class_factory_and_plugin_are_released() {
        unsafe {
            let mut factory = std::ptr::null_mut();
            assert_eq!(
                DllGetClassObject(
                    &CLSID_WINMERGESCRIPT,
                    &IClassFactory::IID,
                    &mut factory,
                ),
                S_OK
            );
            assert_eq!(GLOBAL_OBJECTS.load(Ordering::Acquire), 1);

            let mut plugin = std::ptr::null_mut();
            assert_eq!(
                cf_create_instance(
                    factory,
                    std::ptr::null_mut(),
                    &IID_IWINMERGESCRIPT,
                    &mut plugin,
                ),
                S_OK
            );
            assert_eq!(GLOBAL_OBJECTS.load(Ordering::Acquire), 2);

            assert_eq!(wm_release(plugin), 0);
            assert_eq!(cf_release(factory), 0);
            assert_eq!(DllCanUnloadNow(), S_OK);
        }
    }
}
