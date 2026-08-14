use std::{
    collections::HashMap,
    env::var_os,
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
            CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HINSTANCE, LPARAM, LRESULT, WPARAM,
        },
        System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_CONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL,
                VK_RMENU, VK_RSHIFT, VK_RWIN,
            },
            WindowsAndMessaging::{
                CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, MB_ICONERROR,
                MB_OK, MSG, MessageBoxW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
                WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    },
    core::PCWSTR,
};

use crate::{
    activator,
    config::{Config, virtual_key},
    decoder::{Action, Decoder},
};

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
        activator::open_shell(&path.to_string_lossy());
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
            activator::open_or_activate(&target);
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
