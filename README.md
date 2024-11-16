dprint 0.47.2
Copyright 2020-2023 by David Sherret

Auto-formats source code based on the specified plugins.

USAGE:
    dprint <SUBCOMMAND> [OPTIONS] [--] [file patterns]...

SUBCOMMANDS:
  init                    Initializes a configuration file in the current directory.
  fmt                     Formats the source files and writes the result to the file system.
  check                   Checks for any files that haven't been formatted.
  config                  Functionality related to the configuration file.
  output-file-paths       Prints the resolved file paths for the plugins based on the args and configuration.
  output-resolved-config  Prints the resolved configuration for the plugins based on the args and configuration.
  output-format-times     Prints the amount of time it takes to format each file. Use this for debugging.
  clear-cache             Deletes the plugin cache directory.
  upgrade                 Upgrades the dprint executable.
  completions             Generate shell completions script for dprint
  license                 Outputs the software license.
  lsp                     Starts up a language server for formatting files.

More details at `dprint help <SUBCOMMAND>`

OPTIONS:
  -c, --config <config>          Path or url to JSON configuration file. Defaults to dprint.json(c) or .dprint.json(c) in current or ancestor directory when not provided.
      --plugins <urls/files>...  List of urls or file paths of plugins to use. This overrides what is specified in the config file.
  -L, --log-level <log-level>    Set log level [default: info] [possible values: debug, info, warn, error, silent]

ENVIRONMENT VARIABLES:
  DPRINT_CACHE_DIR     Directory to store the dprint cache. Note that this
                       directory may be periodically deleted by the CLI.
  DPRINT_MAX_THREADS   Limit the number of threads dprint uses for
                       formatting (ex. DPRINT_MAX_THREADS=4).
  DPRINT_CERT          Load certificate authority from PEM encoded file.
  DPRINT_TLS_CA_STORE  Comma-separated list of order dependent certificate stores.
                       Possible values: "mozilla" and "system".
                       Defaults to "mozilla,system".
  HTTPS_PROXY          Proxy to use when downloading plugins or configuration
                       files (set HTTP_PROXY for HTTP).

GETTING STARTED:
  1. Navigate to the root directory of a code repository.
  2. Run `dprint init` to create a dprint.json file in that directory.
  3. Modify configuration file if necessary.
  4. Run `dprint fmt` or `dprint check`.

EXAMPLES:
  Write formatted files to file system:

    dprint fmt

  Check for files that haven't been formatted:

    dprint check

  Specify path to config file other than the default:

    dprint fmt --config path/to/config/dprint.json

  Search for files using the specified file patterns:

    dprint fmt "**/*.{ts,tsx,js,jsx,json}"

Latest version: 0.47.5 (Current is 0.47.2)
Download the latest version by running: dprint upgrade
