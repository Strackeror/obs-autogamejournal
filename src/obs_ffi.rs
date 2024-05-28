use anyhow::{bail, Context, Result};
use obs_sys::bfree;
use obs_sys::blog;
use obs_sys::obs_frontend_add_event_callback;
use obs_sys::obs_frontend_get_current_record_output_path;
use obs_sys::obs_frontend_get_last_replay;
use obs_sys::obs_frontend_get_last_screenshot;
use obs_sys::obs_frontend_replay_buffer_active;
use obs_sys::obs_frontend_replay_buffer_save;
use obs_sys::obs_frontend_replay_buffer_start;
use obs_sys::obs_frontend_replay_buffer_stop;
use obs_sys::obs_frontend_take_screenshot;
use obs_sys::{LOG_DEBUG, LOG_ERROR, LOG_INFO, LOG_WARNING};
use std::ffi::{c_char, c_void, CStr};
use std::panic;
use std::path::PathBuf;
use std::str::FromStr;

pub enum LogLevel {
    Debug = LOG_DEBUG as _,
    Info = LOG_INFO as _,
    Warn = LOG_WARNING as _,
    Error = LOG_ERROR as _,
}

pub fn log(level: LogLevel, text: &str) {
    unsafe {
        blog(
            level as i32,
            "%s\0".as_ptr() as *const c_char,
            format!("[{}]{}\0", "obs_clip", text).as_ptr() as *const c_char,
        )
    }
}

pub fn add_event_callback<T>(func: &'static T)
where
    T: Fn(i32),
    T: Sync,
    T: panic::RefUnwindSafe,
{
    unsafe extern "C" fn cb<U>(event_type: i32, closure: *mut c_void)
    where
        U: Fn(i32),
        U: Sync,
        U: panic::RefUnwindSafe,
    {
        let closure = &mut *(closure as *mut U);
        closure(event_type)
    }

    unsafe { obs_frontend_add_event_callback(Some(cb::<T>), func as *const _ as _) }
}

struct OwnedPtr(*mut c_char);
impl Drop for OwnedPtr {
    fn drop(&mut self) {
        unsafe { bfree(self.0 as _) };
    }
}

pub fn get_last_screenshot_path() -> Result<PathBuf> {
    let char_ptr = OwnedPtr(unsafe { obs_frontend_get_last_screenshot() });
    if char_ptr.0.is_null() {
        bail!("No last screenshot");
    }

    let str = unsafe { CStr::from_ptr(char_ptr.0) }.to_str()?;
    PathBuf::from_str(str).context("converting to path")
}

pub fn get_last_replay_path() -> Result<PathBuf> {
    let char_ptr = OwnedPtr(unsafe { obs_frontend_get_last_replay() });
    if char_ptr.0.is_null() {
        bail!("No last replay");
    }

    let str = unsafe { CStr::from_ptr(char_ptr.0) }.to_str()?;
    PathBuf::from_str(str).context("converting to path")
}

pub fn get_current_record_output_path() -> Result<PathBuf> {
    let char_ptr = OwnedPtr(unsafe { obs_frontend_get_current_record_output_path() });
    if char_ptr.0.is_null() {
        bail!("No profile path");
    }

    let str = unsafe { CStr::from_ptr(char_ptr.0) }.to_str()?;
    PathBuf::from_str(str).context("converting to path")
}

pub fn take_screnshot() {
    unsafe { obs_frontend_take_screenshot() };
}

pub fn save_replay() {
    unsafe { obs_frontend_replay_buffer_save() };
}

pub fn start_replay_buffer() {
    unsafe { obs_frontend_replay_buffer_start() }
}

pub fn stop_replay_buffer() {
    unsafe { obs_frontend_replay_buffer_stop() }
}

pub fn replay_buffer_active() -> bool {
    unsafe { obs_frontend_replay_buffer_active() }
}