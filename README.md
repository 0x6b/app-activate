# app-activate

A minimal application launcher, just for my needs.

## Features

- Two-shot global hotkeys to launch or activate an app, with the option to log to an SQLite database
- Text-based configuration
- No GUI

## Usage

```console
$ app-activate --help
Usage: app-activate [OPTIONS] [COMMAND]

Commands:
  start       Start the application. Default if no subcommand is provided
  register    Register the application to start on login
  unregister  Unregister the application from starting on login
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  Path to the configuration file. Defaults to
                         `$XDG_CONFIG_HOME/app-activate/config.toml`
  -h, --help             Print help
  -V, --version          Print version
```

## How to Install

```console
$ cargo install --git https://github.com/0x6b/app-activate
```

## How to Update

```console
$ git switch main
$ git pull
$ cargo x update
```

## How to Configure

Place the configuration file at `$XDG_CONFIG_HOME/app-activate/config.toml`. If `$XDG_CONFIG_HOME` is not set, it defaults to `~/.config/app-activate/config.toml`.

```console
$ CONFIG_ROOT=~/.config/app-activate
$ mkdir -p $CONFIG_ROOT
$ curl -o- https://raw.githubusercontent.com/0x6b/app-activate/refs/heads/main/config.toml > $CONFIG_ROOT/config.toml
$ $EDITOR $CONFIG_ROOT/config.toml
```

Configure the hotkeys and applications as you like. After the launch, the changes will be picked up automatically. See the [keyboard-types](https://github.com/pyfisch/keyboard-types/blob/v0.7.0/src/key.rs#L991) crate for available keycodes. No modifier keys are supported.

## How to Use as a System Service

You can use this as a CLI application (the classic UNIX job control method, i.e., `app-activate &`), but you can also run it as a system service. At this moment, it's working on macOS only. Tested on macOS 15.0.1 Sequoia.

```sh
$ app-activate register
$ ps -ef | grep app-activate # ~/.cargo/bin/app-activate should be running
```

The `register` subcommand expects that:

- the binary to be in the `~/.cargo/bin/app-activate`.
- the configuration file to be in the `$XDG_CONFIG_HOME/app-activate/config.toml`.

Yes, these are hardcoded.

## How to Uninstall

```sh
$ app-activate unregister # if you have registered it as a system service
$ cargo uninstall app-activate
```

## Reporting Launch History

If you have configured `db` in the configuration file, launch history will be logged to the SQLite database. You can query the database to see the launch history. The schema is as follows:

```sql
CREATE TABLE log (
  datetime INTEGER NOT NULL, -- UNIX timestamp
  application TEXT NOT NULL  -- path to the application
);
```

Or you can use the `app-activate-reporter` to see the launch history.

```console
$ app-activate-reporter
 Today                      Last 7 days                Last 30 days             
 2024-11-28 → 2024-11-28    2024-11-21 → 2024-11-28    2024-10-29 → 2024-11-28  
| Application   | Count |  | Application   | Count |  | Application   | Count |
| ------------- | ----: |  | ------------- | ----: |  | ------------- | ----: |
| Ghostty       |     5 |  | Slack         |   266 |  | Slack         |   990 |
| Slack         |     3 |  | Google Chrome |   172 |  | Firefox       |   725 |
| Google Meet   |     2 |  | Firefox       |   155 |  | Ghostty       |   560 |
| Firefox       |     2 |  | Ghostty       |   152 |  | Google Chrome |   549 |
| RustRover     |     1 |  | Wezlix        |    50 |  | Wezlix        |   304 |
| Google Chrome |     1 |  | RustRover     |    44 |  | RustRover     |   111 |
| -             |     - |  | Google Meet   |    34 |  | Google Meet   |    98 |
| -             |     - |  | Calendar      |    25 |  | Notion        |    90 |
| -             |     - |  | Zed           |    18 |  | Calendar      |    69 |
| -             |     - |  | Notion        |    16 |  | Zed           |    58 |
```

## How to Contribute

This is my launcher. I’ll maintain it as long as it meets my needs, or until I find a better alternative. I’m not looking for contributions, but I’m sharing the code in case it helps someone else. Please feel free to fork it and modify it however you like. I'm not interested in making this:

- more capable
- more configurable
- more user-friendly
- more attractive
- more popular
- GUI-based
- cross-platform (beyond my future use)

There should be similar and/or more capable tools available in every language and platform, so if you have a better option, feel free to keep using that.

## Motivation

I'm a big fan of [Apptivate](http://www.apptivateapp.com/) (macOS app) as it allows me to quickly launch apps using keyboard shortcuts. It's a simple, beautiful way to create global hotkeys for my applications, as exactly advertised.

However, the last update was back in [2020-12-29](https://x.com/apptivateapp/status/1343810481417551872) and the future of the app is uncertain. Although, at this time of writing, it's totally working fine on my macOS 15.0.1 Sequoia, I wanted to create a plan B in case it stops working in the future. This repository is my attempt to create a similar app using Rust, just solely for my own use case.

## License

MIT. See [LICENSE](LICENSE) for more details.
