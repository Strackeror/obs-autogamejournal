use std::{cell::RefCell, ffi::c_char, ptr::null_mut};

use obs_sys::{
    blog, obs_module_t, LIBOBS_API_MAJOR_VER, LIBOBS_API_MINOR_VER, LIBOBS_API_PATCH_VER,
    LOG_DEBUG, LOG_ERROR, LOG_INFO, LOG_WARNING,
};

enum LogLevel {
    Debug = LOG_DEBUG as _,
    Info = LOG_INFO as _,
    Warn = LOG_WARNING as _,
    Error = LOG_ERROR as _,
}

fn obs_log(level: LogLevel, text: &str) {
    unsafe {
        blog(
            level as i32,
            "%s\0".as_ptr() as *const c_char,
            format!("[{}]{}\0", "obs_clip", text).as_ptr() as *const c_char,
        )
    }
}

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    obs_log(LogLevel::Info, "Toast");
    true
}

thread_local! {
static MODULE: RefCell<*mut obs_module_t> = const {RefCell::new(null_mut())};
}
#[no_mangle]
pub extern "C" fn obs_module_set_pointer(ptr: *mut obs_module_t) {
    MODULE.with_borrow_mut(|v| *v = ptr);
}

#[no_mangle]
pub extern "C" fn obs_module_ver() -> u32 {
    LIBOBS_API_MAJOR_VER << 24 | LIBOBS_API_MINOR_VER << 16 | LIBOBS_API_PATCH_VER
}
