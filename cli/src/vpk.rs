use anyhow::{Result as AnyResult, bail};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr::{self, null_mut};

#[link(name = "vpkinfo", kind = "raw-dylib")]
unsafe extern "C" {
    #[link_name = "FreeString"]
    unsafe fn free_string(string: *const c_char);
    #[link_name = "GetLastErrorMessage"]
    unsafe fn get_last_error_message() -> *mut c_char;
    #[link_name = "CreateVpk"]
    unsafe fn create_vpk(path: *const c_char, handle: *mut *mut c_void) -> c_int;
    #[link_name = "DestroyVpk"]
    unsafe fn destroy_vpk(handle: *mut c_void);
    #[link_name = "GetAddonInfoContent"]
    unsafe fn get_addoninfo_content(handle: *mut c_void, content: *mut *mut c_char) -> c_int;
    #[link_name = "GetMissionContent"]
    unsafe fn get_mission_content(handle: *mut c_void, content: *mut *mut c_char) -> c_int;
}

/// 该函数会调用free_string释放c_str
fn cstr_to_string(c_str: *const c_char) -> String {
    unsafe {
        let str = CStr::from_ptr(c_str).to_string_lossy().into_owned();
        // 释放c_str资源
        free_string(c_str);
        str
    }
}

#[derive(Debug)]
pub struct VPKInfo(*mut c_void);

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        let path = CString::new(path.as_ref().to_str().unwrap())?;
        let mut handle: *mut c_void = ptr::null_mut();
        let result = unsafe { create_vpk(path.as_ptr(), &mut handle as *mut _) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if handle == ptr::null_mut() {
            bail!("Failed to create vpk object");
        }
        Ok(Self(handle))
    }

    fn get_last_error_message() -> String {
        let ptr = unsafe { get_last_error_message() };
        cstr_to_string(ptr)
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        let mut content_ptr: *mut c_char = null_mut();
        let result = unsafe { get_addoninfo_content(self.0, &mut content_ptr) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if content_ptr == ptr::null_mut() {
            bail!("该文件没有addoninfo信息");
        }

        let content = cstr_to_string(content_ptr);
        Ok(content)
    }

    pub fn get_mission(&self) -> AnyResult<String> {
        let mut content_ptr: *mut c_char = null_mut();
        let result = unsafe { get_mission_content(self.0, &mut content_ptr) };
        if result != 0 {
            let err_msg = VPKInfo::get_last_error_message();
            bail!(err_msg);
        }
        if content_ptr == ptr::null_mut() {
            bail!("该文件没有misson信息");
        }

        let content = cstr_to_string(content_ptr);
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
