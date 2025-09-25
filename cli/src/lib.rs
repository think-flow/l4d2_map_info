use anyhow::{Result as AnyResult, bail};
use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;

pub struct VPKInfo {
    lib: Library,
    vpk: *mut c_void,
}

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        unsafe {
            let lib = Library::new("vpkinfo.dll")?;
            let path = CString::new(path.as_ref().to_str().unwrap())?;

            let create_vpk: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
                lib.get(b"create_vpk")?;
            let vpk = create_vpk(path.as_ptr());
            if vpk.is_null() {
                bail!("Failed to create vpk object");
            }

            Ok(Self { lib: lib, vpk: vpk })
        }
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        unsafe {
            let get_addoninfo_content_length: Symbol<unsafe extern "C" fn(*mut c_void) -> c_uint> =
                self.lib.get(b"get_addoninfo_content_length")?;
            let get_addoninfo_content: Symbol<
                unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
            > = self.lib.get(b"get_addoninfo_content")?;

            let length = get_addoninfo_content_length(self.vpk);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = get_addoninfo_content(self.vpk, buf.as_mut_ptr(), buf.len() as c_int);

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
    }

    pub fn get_mission(&self) -> AnyResult<String> {
        unsafe {
            let get_mission_content_length: Symbol<unsafe extern "C" fn(*mut c_void) -> c_uint> =
                self.lib.get(b"get_mission_content_length")?;
            let get_mission_content: Symbol<
                unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
            > = self.lib.get(b"get_mission_content")?;

            let length = get_mission_content_length(self.vpk);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = get_mission_content(self.vpk, buf.as_mut_ptr(), buf.len() as c_int);

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
    }
}

impl Drop for VPKInfo {
    fn drop(&mut self) {
        unsafe {
            if let Ok(destroy_vpk) = self
                .lib
                .get::<unsafe extern "C" fn(*mut c_void)>(b"destroy_vpk")
            {
                destroy_vpk(self.vpk);
            }
        }
    }
}
