# app-activate

I'm a big fan of [Apptivate](http://www.apptivateapp.com/) (macOS app) as it allows me to quickly launch apps using keyboard shortcuts. It's a simple, beautiful way to create global hotkeys for my applications, as exactly advertised.

However, the last update was back in [2020-12-29](https://x.com/apptivateapp/status/1343810481417551872) and the future of the app is uncertain. Although, at this time of writing, it's totally working fine on my macOS 15.0.1 Sequoia, I wanted to create a plan B in case it stops working in the future. This repository is my attempt to create a similar app using Rust, just solely for my own use case.

## Features

- Two-shot global hotkeys to launch or activate apps
- Simple configuration file

## How to Install

```sh
$ git clone https://github.com/0x6b/app-activate
$ cd app-activate
$ cargo install --path .
$ mkdir -p ~/.config/app-activate
$ cp config.toml ~/.config/app-activate
$ app-activate register
$ ps -ef | grep app-activate # ~/.cargo/bin/app-activate should be running
```

## How to Configure

Edit `~/.config/app-activate/config.toml` to add your own hotkeys and apps. After the launch, the changes will be picked up automatically.

```toml
leader_key = "F10" # A hotkey to trigger the launcher
timeout_ms = 600 # The time in milliseconds to wait for the next key press

# A pair of second hotkey and absolute path to the application
[applications]
c = "/System/Applications/Calendar.app" # means that pressing the `leader_key`,
                                        # (releasing it), and then pressing `c`
                                        # within the `timeout_ms` will open
                                        # the Calendar app
f = "/Applications/Firefox.app"
g = "/Applications/Google Chrome.app"
i = "/System/Library/CoreServices/Finder.app"
s = "/Applications/Slack.app"
t = "/Applications/Ghostty.app"
```

## How to Uninstall

```sh
$ app-activate unregister
$ cargo uninstall app-activate
```

## Contributing

This is my launcher. I’ll maintain it as long as it meets my needs, or until I find a better alternative. I’m not looking for contributions, but I’m sharing the code in case it helps someone else. Please feel free to fork it and modify it however you like. I'm not interested in making this:

- more configurable
- more user-friendly
- more attractive
- more popular
- GUI-based
- cross-platform (beyond my future use)

There should be similar and/or more capable tools available in every language and platform, so if you have a better option, feel free to keep using that.

## License

MIT. See [LICENSE](LICENSE) for more details.
