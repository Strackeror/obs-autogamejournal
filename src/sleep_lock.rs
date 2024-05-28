use anyhow::Result;
use retour::GenericDetour;
use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use winsafe::prelude::*;
use winsafe::HINSTANCE;

use crate::obs_ffi::log;
use crate::obs_ffi::replay_buffer_active;
use crate::obs_ffi::start_replay_buffer;
use crate::obs_ffi::stop_replay_buffer;
use crate::obs_ffi::LogLevel;

type FnSleepInhibit = extern "C" fn(*mut c_void, bool);

extern "C" fn detour_func(_: *mut c_void, _: bool) {}

static DETOUR: Mutex<Option<GenericDetour<FnSleepInhibit>>> = Mutex::new(None);

pub fn disable_inhibit_sleep() -> Result<()> {
    let lib = HINSTANCE::LoadLibrary("obs.dll")?;
    let func: FnSleepInhibit =
        unsafe { std::mem::transmute(lib.GetProcAddress("os_inhibit_sleep_set_active")?) };
    let detour = unsafe { GenericDetour::new(func, detour_func) }?;
    unsafe { detour.enable()? };

    *DETOUR.lock().unwrap() = Some(detour);
    Ok(())
}

fn restart_replay_buffer() {
    log(LogLevel::Info, "Restarting Replay buffer");
    while replay_buffer_active() {
        std::thread::sleep(Duration::from_secs(5));
        stop_replay_buffer();
    }

    while !replay_buffer_active() {
        std::thread::sleep(Duration::from_secs(5));
        start_replay_buffer();
    }
}


pub fn replay_buffer_restart_thread() {
    let mut last_time = Instant::now();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let cur_time = Instant::now();
        if cur_time - last_time > Duration::from_secs(5) {
            restart_replay_buffer();
        }
        last_time = Instant::now();
    }
}
