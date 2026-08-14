use std::{
    collections::HashMap,
    env::{current_dir, var_os},
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        mpsc::{Sender, channel},
    },
    thread::spawn,
    time::Instant,
};

use windows::{
    Win32::{
        Foundation::{
            APPMODEL_ERROR_NO_APPLICATION, CloseHandle, ERROR_ALREADY_EXISTS,
            ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, GetLastError, HINSTANCE, HWND, LPARAM,
            LRESULT, WPARAM,
        },
        Storage::Packaging::Appx::GetApplicationUserModelId,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{
                AttachThreadInput, CreateMutexW, GetCurrentThreadId, OpenProcess,
                PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_CONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL,
                VK_RMENU, VK_RSHIFT, VK_RWIN,
            },
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                BringWindowToTop, CallNextHookEx, DispatchMessageW, EnumWindows, GWL_EXSTYLE,
                GetForegroundWindow, GetMessageW, GetWindowLongW, GetWindowThreadProcessId,
                IsWindowVisible, KBDLLHOOKSTRUCT, MB_ICONERROR, MB_OK, MSG, MessageBoxW,
                SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, SetWindowsHookExW, ShowWindow,
                TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
                WM_SYSKEYDOWN, WM_SYSKEYUP, WS_EX_TOOLWINDOW,
            },
        },
    },
    core::{BOOL, PCWSTR, PWSTR},
};

use crate::{
    config::{Config, virtual_key},
    decoder::{Action, Decoder},
    executable,
};

#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($argument:tt)*) => {{
        eprintln!("[app-activate] {}", format_args!($($argument)*))
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($argument:tt)*) => {{
        if false {
            let _ = format_args!($($argument)*);
        }
    }};
}

static DECODER: OnceLock<Mutex<Decoder>> = OnceLock::new();
static LAUNCHER: OnceLock<Sender<String>> = OnceLock::new();

pub fn run() -> Result<(), String> {
    let instance_name = wide("Local\\app-activate.Windows");
    let instance = unsafe { CreateMutexW(None, false, PCWSTR(instance_name.as_ptr())) }
        .map_err(|error| format!("Could not create the single-instance lock: {error}"))?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(instance);
        }
        return Ok(());
    }

    let path = config_path()?;
    if !path.exists() {
        create_config(&path)?;
        open_shell(&path.to_string_lossy());
        return Ok(());
    }
    let config = Config::load(&path)?;
    let leader = virtual_key(&config.launcher.leader)?;
    let mappings = config
        .launcher
        .primary
        .iter()
        .map(|(key, target)| virtual_key(key).map(|key| (key, target.clone())))
        .collect::<Result<HashMap<_, _>, _>>()?;
    debug_log!(
        "loaded {} mapping(s) from {}; leader=0x{leader:02X}",
        mappings.len(),
        path.display()
    );
    DECODER
        .set(Mutex::new(Decoder::new(leader, config.launcher.timeout_ms, mappings)))
        .map_err(|_| "launcher was already initialized".to_string())?;

    let (sender, receiver) = channel();
    LAUNCHER
        .set(sender)
        .map_err(|_| "launcher worker was already initialized".to_string())?;
    spawn(move || {
        while let Ok(target) = receiver.recv() {
            debug_log!("resolved launcher target: {target}");
            open_or_activate(&target);
        }
    });

    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| format!("Could not get executable module handle: {error}"))?;
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), Some(HINSTANCE(module.0)), 0)
    }
    .map_err(|error| format!("Could not install keyboard hook: {error}"))?;
    debug_log!("keyboard hook installed");
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    unsafe {
        let _ = UnhookWindowsHookEx(hook);
        let _ = CloseHandle(instance);
    }
    Ok(())
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        if let Some(lock) = DECODER.get() {
            if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
                let modified = modifier_down();
                if let Ok(mut decoder) = lock.lock() {
                    match decoder.key_down(event.vkCode, modified, Instant::now()) {
                        Action::Consume => return LRESULT(1),
                        Action::Launch(target) => {
                            if let Some(sender) = LAUNCHER.get() {
                                let _ = sender.send(target);
                            }
                            return LRESULT(1);
                        }
                        Action::PassThrough => {}
                    }
                }
            } else if (message == WM_KEYUP || message == WM_SYSKEYUP)
                && lock.lock().is_ok_and(|mut decoder| decoder.key_up(event.vkCode))
            {
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn modifier_down() -> bool {
    [VK_CONTROL, VK_LMENU, VK_MENU, VK_LSHIFT, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_LWIN, VK_RWIN]
        .iter()
        .any(|key| unsafe { GetAsyncKeyState(key.0 as i32) } < 0)
}

fn config_path() -> Result<PathBuf, String> {
    let root = var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(root).join("app-activate").join("config.toml"))
}

fn create_config(path: &Path) -> Result<(), String> {
    create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    write(path, include_str!("../config.example.toml")).map_err(|error| error.to_string())
}

fn open_or_activate(target: &str) {
    if let Some(app_id) = packaged_app_id(target) {
        debug_log!("opening packaged target; app_id={app_id}");
        if activate_packaged_app(app_id) {
            return;
        }
        debug_log!("no running packaged window activated; falling back to ShellExecuteW");
        open_shell(target);
        return;
    }

    let path = Path::new(target);
    let executable = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    debug_log!("opening target; executable={executable}, target={target}");
    if executable {
        if activate_executable(path) {
            return;
        }
        debug_log!("no running window activated; falling back to ShellExecuteW");
    }
    open_shell(target);
}

fn packaged_app_id(target: &str) -> Option<&str> {
    const PREFIX: &str = "shell:AppsFolder\\";
    let (prefix, app_id) = target.split_at_checked(PREFIX.len())?;
    (prefix.eq_ignore_ascii_case(PREFIX) && !app_id.is_empty()).then_some(app_id)
}

fn activate_executable(executable: &Path) -> bool {
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
                    let matched = executable::matches(&path, executable);
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

fn open_shell(target: &str) {
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

pub fn show_error(message: &str) {
    let message = wide(message);
    let title = wide("app-activate");
    unsafe {
        MessageBoxW(None, PCWSTR(message.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
