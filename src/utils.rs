use std::ffi::CStr;

pub fn c_bytesto_string(buf: &[i8]) -> String {
    let c_str = unsafe {
        let ptr = buf.as_ptr() as *const i8;
        CStr::from_ptr(ptr)
    };

    c_str.to_string_lossy().into_owned()
}