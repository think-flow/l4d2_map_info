use anyhow::{Result as AnyResult, bail};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;

#[link(name = "vpkinfo", kind = "raw-dylib")]
unsafe extern "C" {
    fn create_vpk(path: *const c_char) -> *mut c_void;
    fn destroy_vpk(handle: *mut c_void);
    fn get_addoninfo_content_length(handle: *mut c_void) -> c_uint;
    fn get_mission_content_length(handle: *mut c_void) -> c_uint;
    fn get_addoninfo_content(handle: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
    fn get_mission_content(handle: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
}

#[derive(Debug)]
pub struct VPKInfo(*mut c_void);

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        let path = CString::new(path.as_ref().to_str().unwrap())?;
        let vpk = unsafe { create_vpk(path.as_ptr()) };
        if vpk.is_null() {
            bail!("Failed to create vpk object");
        }
        Ok(Self(vpk))
    }

    fn get_content(
        &self,
        len_fn: unsafe extern "C" fn(*mut c_void) -> c_uint,
        content_fn: unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
    ) -> AnyResult<String> {
        let length = unsafe { len_fn(self.0) };
        if length == 0 {
            return Ok("".to_string());
        }
        let mut buf = vec![0u8; length as usize];
        let copied = unsafe { content_fn(self.0, buf.as_mut_ptr(), buf.len() as c_int) };
        if copied <= 0 {
            match copied {
                0 => return Ok("".to_string()),
                -1 => bail!("handle is null"),
                -2 => bail!("buffer is null"),
                -3 => bail!("buffer size is too small"),
                _ => bail!("unknown error"),
            }
        }
        Ok(String::from_utf8_lossy(&buf[..copied as usize]).into_owned())
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        self.get_content(get_addoninfo_content_length, get_addoninfo_content)
    }

    pub fn get_mission(&self) -> AnyResult<String> {
        self.get_content(get_mission_content_length, get_mission_content)
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
