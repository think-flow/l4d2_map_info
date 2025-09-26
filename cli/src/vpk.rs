use anyhow::{Result as AnyResult, bail};
use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::sync::LazyLock;

// 运行时加载外部 vpkinfo.dll
static LIBRARY: LazyLock<Library> =
    LazyLock::new(|| unsafe { Library::new("vpkinfo.dll").expect("Failed to load vpkinfo.dll") });

/*
* 外部函数列表
* create_vpk
* destroy_vpk
* get_addoninfo_content_length
* get_mission_content_length
* get_addoninfo_content
* get_mission_content
*/
static CREATE_VPK: LazyLock<Symbol<'static, unsafe extern "C" fn(*const c_char) -> *mut c_void>> =
    LazyLock::new(|| unsafe {
        LIBRARY
            .get(b"create_vpk")
            .expect("Failed to load create_vpk function")
    });

static DESTROY_VPK: LazyLock<Symbol<'static, unsafe extern "C" fn(*mut c_void)>> =
    LazyLock::new(|| unsafe {
        LIBRARY
            .get(b"destroy_vpk")
            .expect("Failed to load destroy_vpk function")
    });

static GET_ADDONINFO_CONTENT_LENGTH: LazyLock<
    Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_uint>,
> = LazyLock::new(|| unsafe {
    LIBRARY
        .get(b"get_addoninfo_content_length")
        .expect("Failed to load get_addoninfo_content_length function")
});

static GET_MISSION_CONTENT_LENGTH: LazyLock<
    Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_uint>,
> = LazyLock::new(|| unsafe {
    LIBRARY
        .get(b"get_mission_content_length")
        .expect("Failed to load get_mission_content_length function")
});

static GET_ADDONINFO_CONTENT: LazyLock<
    Symbol<'static, unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int>,
> = LazyLock::new(|| unsafe {
    LIBRARY
        .get(b"get_addoninfo_content")
        .expect("Failed to load get_addoninfo_content function")
});

static GET_MISSION_CONTENT: LazyLock<
    Symbol<'static, unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int>,
> = LazyLock::new(|| unsafe {
    LIBRARY
        .get(b"get_mission_content")
        .expect("Failed to load get_mission_content function")
});

pub struct VPKInfo(*mut c_void);

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> AnyResult<Self> {
        unsafe {
            let path = CString::new(path.as_ref().to_str().unwrap())?;
            let vpk = CREATE_VPK(path.as_ptr());
            if vpk.is_null() {
                bail!("Failed to create vpk object");
            }

            Ok(Self(vpk))
        }
    }

    pub fn get_addoninfo(&self) -> AnyResult<String> {
        unsafe {
            let length = GET_ADDONINFO_CONTENT_LENGTH(self.0);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = GET_ADDONINFO_CONTENT(self.0, buf.as_mut_ptr(), buf.len() as c_int);

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
            let length = GET_MISSION_CONTENT_LENGTH(self.0);
            if length == 0 {
                return Ok("".to_string());
            }

            let mut buf = vec![0u8; length as usize];
            let copied = GET_MISSION_CONTENT(self.0, buf.as_mut_ptr(), buf.len() as c_int);

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
            DESTROY_VPK(self.0);
        }
    }
}
