# app-activate

A small native Windows leader-key launcher. Press the configured leader key,
then a mapped key. Executables are brought to the foreground when already open
and launched otherwise; URLs open with their default handler.

The original macOS app in this repository was superseded by
[Switch](https://github.com/0x6b/switch). This repository has been revived for
the Windows launcher developed there.

## Scope

Open-or-activate mappings cover most navigation without a window list. Windows
keeps standard <kbd>Alt</kbd>+<kbd>Tab</kbd> behavior, while
[PowerToys Run's Window Walker](https://learn.microsoft.com/en-us/windows/powertoys/run#window-walker-plugin)
handles occasional title-based searches with its `<` command. app-activate is
deliberately limited to one leader-key mapping set, executable/URL/packaged-app
targets, and TOML configuration; it has no GUI, installer, or config auto-reload.

## Requirements

- Windows 11
- Rust stable with the `x86_64-pc-windows-msvc` toolchain
- MSVC v143 x64/x86 Build Tools and a Windows 11 SDK (Visual Studio itself is
  not required)

## Build and run

```powershell
cargo build --release
.\target\release\app-activate.exe
```

For a checkout accessed through a `\\wsl.localhost\...` path, put Cargo output
on the Windows filesystem because WSL does not support incremental compilation's
lock:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\app-activate\cargo-target"
cargo run
```

Set `CARGO_TARGET_DIR` in your PowerShell profile to persist it. Release output
then lives at `$env:CARGO_TARGET_DIR\release\app-activate.exe`.

Debug builds (`cargo run`) remain attached to the console, stop with
<kbd>Ctrl</kbd>+<kbd>C</kbd>, and log target resolution, process matching, and
activation results. Release builds use the Windows GUI subsystem without a
console window.

On first run app-activate creates and opens
`%LOCALAPPDATA%\app-activate\config.toml`. Edit its mappings and run the program
again. Stop it through Task Manager or `Stop-Process app-activate`.

## Install, update, and start at login

Build and copy the release executable to a stable location. Repeat these same
commands to update it:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\app-activate\cargo-target"
cargo build --release

Stop-Process -Name app-activate -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\app-activate" | Out-Null
Copy-Item `
  "$env:CARGO_TARGET_DIR\release\app-activate.exe" `
  "$env:LOCALAPPDATA\app-activate\app-activate.exe"
Start-Process "$env:LOCALAPPDATA\app-activate\app-activate.exe"
```

Create the current user's Startup shortcut:

```powershell
$startup = [Environment]::GetFolderPath("Startup")
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut("$startup\app-activate.lnk")
$shortcut.TargetPath = "$env:LOCALAPPDATA\app-activate\app-activate.exe"
$shortcut.WorkingDirectory = "$env:LOCALAPPDATA\app-activate"
$shortcut.Save()
```

Remove it to disable startup:

```powershell
Remove-Item "$([Environment]::GetFolderPath('Startup'))\app-activate.lnk"
```

## Targets

Use executable paths for classic desktop apps and URLs for the default browser:

```toml
[launcher.primary]
a = 'C:\Program Files\Alacritty\alacritty.exe'
g = 'https://github.com'
```

Set `cwd` in the launcher table to choose the default working directory when an
application is started. Use the expanded mapping form to override it for one
application:

```toml
[launcher]
cwd = 'C:\Users\you'

[launcher.primary]
a = 'C:\Program Files\Alacritty\alacritty.exe'
e = { target = 'C:\Users\you\AppData\Local\Programs\Microsoft VS Code\Code.exe', cwd = 'C:\src\project' }
```

A mapping without its own `cwd` uses the launcher-level value. If neither is
configured, the launcher's working directory is unchanged. Activating an
already-running application does not change its working directory.

For Microsoft Store and other packaged apps, use a stable Application User
Model ID (AUMID), not a versioned `WindowsApps` executable path:

```powershell
Get-StartApps | Where-Object Name -Match "Teams" | Format-Table Name, AppID
```

Prefix the machine's reported `AppID` with `shell:AppsFolder\`:

```toml
[launcher.primary]
t = 'shell:AppsFolder\MSTeams_8wekyb3d8bbwe!MSTeams'
```

Running packaged apps are matched by AUMID and brought to the foreground. If no
matching window exists, the target is opened through Windows Shell. The leader
key and mapped keys are consumed globally. Modified combinations and unmapped
keys pass through; modifiers also cancel an in-progress sequence.

## Project

This remains a narrow personal launcher shared in case it is useful. It is not
intended to become cross-platform, GUI-based, or broadly configurable.

## License

MIT. See [LICENSE](LICENSE).
