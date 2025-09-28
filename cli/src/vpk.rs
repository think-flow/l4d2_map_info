use anyhow::{Result as AnyResult, bail};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

//":vpkinfo.so" 告知链接器，链接 vpkinfo.so 而不是 libvpkinfo.so
#[cfg_attr(target_os = "linux", link(name = ":vpkinfo.so", kind = "dylib"))]
#[cfg_attr(target_os = "windows", link(name = "vpkinfo", kind = "raw-dylib"))]
unsafe extern "C" {
    #[link_name = "FreeString"]
    unsafe fn free_string(string: *const c_char);
    #[link_name = "GetLastErrorMessage"]
    unsafe fn get_last_error_message() -> *const c_char;
    #[link_name = "CreateVpk"]
    unsafe fn create_vpk(path: *const c_char, handle: *const *const c_void) -> c_int;
    #[link_name = "DestroyVpk"]
    unsafe fn destroy_vpk(handle: *const c_void);
    #[link_name = "GetAddonInfoContent"]
    unsafe fn get_addoninfo_content(handle: *const c_void, content: *const *const c_char) -> c_int;
    #[link_name = "GetMissionContent"]
    unsafe fn get_mission_content(handle: *const c_void, content: *const *const c_char) -> c_int;
}

/// 该函数会调用free_string释放c_str
unsafe fn cstr_ptr_to_string(c_str: *const c_char) -> String {
    let temp = unsafe { CStr::from_ptr(c_str) };
    let string = temp.to_string_lossy().into_owned();
    // 释放c_str资源
    unsafe { free_string(c_str) };
    string
}

#[derive(Debug)]
pub struct VPKInfo(*const c_void);

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        let path = CString::new(path.as_ref().to_string_lossy().as_bytes())?;
        let handle: *const c_void = ptr::null();
        let result = unsafe { create_vpk(path.as_ptr(), &handle) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if handle == ptr::null() {
            bail!("Failed to create vpk object");
        }
        Ok(Self(handle))
    }

    fn get_last_error_message() -> String {
        let ptr = unsafe { get_last_error_message() };
        unsafe { cstr_ptr_to_string(ptr) }
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        let content_ptr: *const c_char = ptr::null();
        let result = unsafe { get_addoninfo_content(self.0, &content_ptr) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if content_ptr == ptr::null() {
            bail!("该文件没有addoninfo信息");
        }

        let content = unsafe { cstr_ptr_to_string(content_ptr) };
        Ok(content)
    }

    pub fn get_mission(&self) -> AnyResult<String> {
        let content_ptr: *const c_char = ptr::null();
        let result = unsafe { get_mission_content(self.0, &content_ptr) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if content_ptr == ptr::null() {
            bail!("该文件没有misson信息");
        }

        let content = unsafe { cstr_ptr_to_string(content_ptr) };
        Ok(content)
    }
}

impl Drop for VPKInfo {
    fn drop(&mut self) {
        unsafe {
            destroy_vpk(self.0);
        }
    }
}

unsafe impl Send for VPKInfo {}
