use anyhow::{Result as AnyResult, bail};
use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::sync::LazyLock;

// 运行时加载外部 vpkinfo.dll
static LIBRARY: LazyLock<Library> =
    LazyLock::new(|| unsafe { Library::new("vpkinfo.dll").expect("Failed to load vpkinfo.dll") });

pub struct VPKInfo {
    vpk_obj: *mut c_void,
}

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        unsafe {
            let path = CString::new(path.as_ref().to_str().unwrap())?;
            let create_vpk: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
                LIBRARY.get(b"create_vpk")?;
            let vpk = create_vpk(path.as_ptr());
            if vpk.is_null() {
                bail!("Failed to create vpk object");
            }

            Ok(Self { vpk_obj: vpk })
        }
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        unsafe {
            let get_addoninfo_content_length: Symbol<unsafe extern "C" fn(*mut c_void) -> c_uint> =
                LIBRARY.get(b"get_addoninfo_content_length")?;
            let get_addoninfo_content: Symbol<
                unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
            > = LIBRARY.get(b"get_addoninfo_content")?;

            let length = get_addoninfo_content_length(self.vpk_obj);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = get_addoninfo_content(self.vpk_obj, buf.as_mut_ptr(), buf.len() as c_int);

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
                LIBRARY.get(b"get_mission_content_length")?;
            let get_mission_content: Symbol<
                unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
            > = LIBRARY.get(b"get_mission_content")?;

            let length = get_mission_content_length(self.vpk_obj);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = get_mission_content(self.vpk_obj, buf.as_mut_ptr(), buf.len() as c_int);

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
            if let Ok(destroy_vpk) =
                LIBRARY.get::<unsafe extern "C" fn(*mut c_void)>(b"destroy_vpk")
            {
                destroy_vpk(self.vpk_obj);
            }
        }
    }
}
