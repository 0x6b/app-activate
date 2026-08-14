use std::{env::current_dir, path::PathBuf};

use windows::{
    Win32::{
        Foundation::{
            APPMODEL_ERROR_NO_APPLICATION, CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS,
            HWND, LPARAM,
        },
        Storage::Packaging::Appx::GetApplicationUserModelId,
        System::Threading::{
            AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                BringWindowToTop, EnumWindows, GWL_EXSTYLE, GetForegroundWindow, GetWindowLongW,
                GetWindowThreadProcessId, IsWindowVisible, SW_RESTORE, SW_SHOWNORMAL,
                SetForegroundWindow, ShowWindow, WS_EX_TOOLWINDOW,
            },
        },
    },
    core::{BOOL, PCWSTR, PWSTR},
};

use crate::target::{self, Target};

pub fn open_or_activate(target: &str) {
    match target::classify(target) {
        Target::PackagedApp(app_id) => {
            debug_log!("opening packaged target; app_id={app_id}");
            if activate_packaged_app(app_id) {
                return;
            }
            debug_log!("no running packaged window activated; falling back to ShellExecuteW");
        }
        Target::Executable(path) => {
            debug_log!("opening target; executable=true, target={target}");
            if activate_executable(path) {
                return;
            }
            debug_log!("no running window activated; falling back to ShellExecuteW");
        }
        Target::Shell(_) => {
            debug_log!("opening target; executable=false, target={target}");
        }
    }
    open_shell(target);
}

fn activate_executable(executable: &std::path::Path) -> bool {
    let expected = if executable.is_absolute() {
        executable.to_path_buf()
    } else {
        current_dir().unwrap_or_default().join(executable)
    };
    activate_window(WindowTarget::Executable(expected))
}

fn activate_packaged_app(app_id: &str) -> bool {
    activate_window(WindowTarget::ApplicationId(app_id.to_owned()))
}

enum WindowTarget {
    Executable(PathBuf),
    ApplicationId(String),
}

fn activate_window(target: WindowTarget) -> bool {
    struct Search {
        target: WindowTarget,
        window: HWND,
    }
    unsafe extern "system" fn callback(window: HWND, data: LPARAM) -> BOOL {
        let search = unsafe { &mut *(data.0 as *mut Search) };
        let visible = unsafe { IsWindowVisible(window) }.as_bool();
        let tool_window =
            unsafe { GetWindowLongW(window, GWL_EXSTYLE) } as u32 & WS_EX_TOOLWINDOW.0 != 0;
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process_id));
        }
        let matched = match &search.target {
            WindowTarget::Executable(executable) => match process_path(process_id) {
                Ok(path) => {
                    let matched = target::executable_matches(&path, executable);
                    debug_log!(
                        "window candidate; pid={process_id}, visible={visible}, tool_window={tool_window}, match={matched}, executable={}",
                        path.display()
                    );
                    matched
                }
                Err(error) => {
                    debug_log!("could not inspect window process {process_id}: {error}");
                    false
                }
            },
            WindowTarget::ApplicationId(expected) => match process_application_id(process_id) {
                Ok(Some(actual)) => {
                    let matched = actual.eq_ignore_ascii_case(expected);
                    debug_log!(
                        "packaged window candidate; pid={process_id}, visible={visible}, tool_window={tool_window}, match={matched}, app_id={actual}"
                    );
                    matched
                }
                Ok(None) => false,
                Err(error) => {
                    debug_log!("could not inspect packaged process {process_id}: {error}");
                    false
                }
            },
        };
        if matched && visible && !tool_window {
            search.window = window;
            return false.into();
        }
        true.into()
    }
    let mut search = Search { target, window: HWND::default() };
    match &search.target {
        WindowTarget::Executable(path) => {
            debug_log!("searching for executable: {}", path.display())
        }
        WindowTarget::ApplicationId(app_id) => {
            debug_log!("searching for packaged app: {app_id}")
        }
    }
    let enumeration =
        unsafe { EnumWindows(Some(callback), LPARAM((&mut search as *mut Search) as isize)) };
    if let Err(error) = enumeration
        && search.window == HWND::default()
    {
        debug_log!("window enumeration stopped: {error}");
    }
    if search.window == HWND::default() {
        debug_log!("no matching visible window found");
        return false;
    }
    unsafe {
        let foreground_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, true).as_bool();
        let _ = ShowWindow(search.window, SW_RESTORE);
        let raised = BringWindowToTop(search.window).is_ok();
        let foreground = SetForegroundWindow(search.window).as_bool();
        if attached {
            let _ = AttachThreadInput(current_thread, foreground_thread, false);
        }
        debug_log!(
            "activation attempted; input_attached={attached}, raised={raised}, foreground={foreground}"
        );
        raised || foreground
    }
}

fn process_path(process_id: u32) -> Result<PathBuf, String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
        .map_err(|error| error.to_string())?;
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    result.map_err(|error| error.to_string())?;
    Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])))
}

fn process_application_id(process_id: u32) -> Result<Option<String>, String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
        .map_err(|error| error.to_string())?;
    let mut length = 0;
    let size_result = unsafe { GetApplicationUserModelId(process, &mut length, None) };
    if size_result == APPMODEL_ERROR_NO_APPLICATION {
        unsafe {
            let _ = CloseHandle(process);
        }
        return Ok(None);
    }
    if size_result != ERROR_INSUFFICIENT_BUFFER {
        unsafe {
            let _ = CloseHandle(process);
        }
        return Err(format!("GetApplicationUserModelId size query failed: {}", size_result.0));
    }

    let mut buffer = vec![0u16; length as usize];
    let result = unsafe {
        GetApplicationUserModelId(process, &mut length, Some(PWSTR(buffer.as_mut_ptr())))
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    if result != ERROR_SUCCESS {
        return Err(format!("GetApplicationUserModelId failed: {}", result.0));
    }
    let value_length = buffer.iter().position(|unit| *unit == 0).unwrap_or(buffer.len());
    Ok(Some(String::from_utf16_lossy(&buffer[..value_length])))
}

pub fn open_shell(target: &str) {
    let target = wide(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    debug_log!("ShellExecuteW returned {}", result.0 as isize);
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
