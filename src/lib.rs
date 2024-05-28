use anyhow::Context;
use anyhow::Result;
use obs_ffi::get_current_record_output_path;
use obs_sys::{obs_module_t, LIBOBS_API_MAJOR_VER, LIBOBS_API_MINOR_VER, LIBOBS_API_PATCH_VER};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::Duration;
use winsafe::GetLastInputInfo;
use winsafe::HWND;

mod obs_ffi;
use obs_ffi::{
    add_event_callback, get_last_replay_path, get_last_screenshot_path, log, take_screnshot,
    LogLevel,
};

mod sleep_lock;

mod window {
    use anyhow::{Context, Result};
    use std::path::Path;
    use winsafe::prelude::*;
    use winsafe::HWND;

    fn normalize_name(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' | ' ' => c,
                _ => '_',
            })
            .collect()
    }

    fn get_process_name_from_pid(pid: u32) -> Result<String> {
        use winsafe::co::TH32CS;
        use winsafe::HPROCESSLIST;
        let mut process_list = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None)?;
        let process = process_list
            .iter_processes()
            .filter_map(|p| p.ok())
            .find(|p| p.th32ProcessID == pid)
            .context("PID not found")?;
        let process_name = Path::new(&process.szExeFile())
            .file_stem()
            .context("Getting file stem")?
            .to_str()
            .context("File to String")?
            .to_owned();
        Ok(process_name)
    }

    pub fn is_window_fullscreen(window: &HWND) -> Result<bool> {
        use winsafe::HMONITOR;

        let rect = window.GetWindowRect()?;
        let monitor = HMONITOR::MonitorFromRect(rect, winsafe::co::MONITOR::DEFAULTTOPRIMARY);
        let monitor_info = monitor.GetMonitorInfo()?;

        if !(rect.left <= monitor_info.rcMonitor.left
            && rect.right >= monitor_info.rcMonitor.right
            && rect.top <= monitor_info.rcMonitor.top
            && rect.bottom >= monitor_info.rcMonitor.bottom)
        {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn get_window_name(window: &HWND) -> Result<String> {
        let (_, pid) = window.GetWindowThreadProcessId();
        if pid == 0 {
            Ok(normalize_name(&window.GetWindowText()?))
        } else {
            get_process_name_from_pid(pid)
        }
    }

    pub fn foreground_window() -> Option<HWND> {
        HWND::GetForegroundWindow()
    }
}

fn check_thread() -> ! {
    let mut last_input_time = 0;
    loop {
        std::thread::sleep(Duration::from_secs(300));

        match GetLastInputInfo() {
            Ok(info) => {
                if info.dwTime <= last_input_time {
                    log(LogLevel::Debug, "No input since last screenshot");
                    continue;
                }
                last_input_time = info.dwTime;
            }
            Err(e) => {
                log(LogLevel::Error, &format!("Failed to get last input: {e:?}"));
            }
        }

        let window = window::foreground_window();
        let Some(window) = window else {
            continue;
        };
        if let Ok(true) = window::is_window_fullscreen(&window) {
            take_screnshot();
        }
    }
}

struct Config {
    target_path: PathBuf,
}

struct Module {
    module_handle: *mut obs_module_t,
    config: Option<Config>,
}
unsafe impl Send for Module {}

static MODULE: Mutex<Module> = Mutex::new(Module {
    module_handle: null_mut(),
    config: None,
});

fn get_target_folder(window: &HWND, config: &Config) -> Result<PathBuf> {
    let title = window::get_window_name(window)?;
    let path = config.target_path.join(title);
    std::fs::create_dir_all(path.clone())?;
    Ok(path)
}

fn replay_saved(config: &Config) -> Result<()> {
    let window = match window::foreground_window() {
        None => return Ok(()),
        Some(w) => w,
    };
    let filename = get_last_replay_path()?;
    let new_filename =
        get_target_folder(&window, config)?.join(filename.file_name().context("get file name")?);
    std::fs::rename(filename, new_filename)?;
    Ok(())
}

fn screenshot_saved(config: &Config) -> Result<()> {
    use image::io::Reader;
    let window = match window::foreground_window() {
        None => return Ok(()),
        Some(w) => w,
    };
    let ss_path = get_last_screenshot_path()?;
    let img = Reader::open(&ss_path)?.decode()?;
    let img = img.as_rgb8().context("conversion to rgb8")?;
    let ss_filename = &ss_path
        .file_name()
        .context("get screenshot name")?
        .to_string_lossy()[11..];

    let new_file = get_target_folder(&window, config)?
        .join(ss_filename)
        .with_extension("jpg");
    img.save_with_format(new_file, image::ImageFormat::Jpeg)?;
    std::fs::remove_file(ss_path)?;
    Ok(())
}

fn event_callback(event_type: i32) {
    let module = MODULE.lock().unwrap();
    let config = module.config.as_ref().unwrap();
    let result = match event_type {
        obs_sys::obs_frontend_event_OBS_FRONTEND_EVENT_REPLAY_BUFFER_SAVED => replay_saved(config),
        obs_sys::obs_frontend_event_OBS_FRONTEND_EVENT_SCREENSHOT_TAKEN => screenshot_saved(config),
        _ => Ok(()),
    };
    if let Err(e) = result {
        log(
            LogLevel::Error,
            &format!("Error handling event {event_type}: {e:?}"),
        )
    }
}

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    MODULE.lock().unwrap().config = Some(Config {
        target_path: get_current_record_output_path().unwrap(),
    });
    add_event_callback(&event_callback);
    sleep_lock::disable_inhibit_sleep().unwrap();
    std::thread::spawn(check_thread);
    std::thread::spawn(sleep_lock::replay_buffer_restart_thread);
    true
}

#[no_mangle]
pub extern "C" fn obs_module_set_pointer(ptr: *mut obs_module_t) {
    MODULE.lock().unwrap().module_handle = ptr;
}

#[no_mangle]
pub extern "C" fn obs_module_ver() -> u32 {
    LIBOBS_API_MAJOR_VER << 24 | LIBOBS_API_MINOR_VER << 16 | LIBOBS_API_PATCH_VER
}
