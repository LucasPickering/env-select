# Changelog

## [1.1.0] - 2024-02-09

### Added

- Pass a list of strings to `multiple` to filter which values are loaded from a mapping

### Changed

- Resolve values in parallel
  - This means profile resolution will only take as long as the slowest step, rather than the sum of all steps

## [1.0.0] - 2024-02-02

### Added

- [A new website!](https://env-select.lucaspickering.me)

### Changed

- Use `es` instead of `env-select` in CLI help output
- `es show config` now accepts optional arguments to print for a single application or profile

## [0.12.0] - 2024-01-26

### Changed

- Update to Rust 1.72 and update dependencies

### Fixed

- Fix log output coloring

### Removed

- [BREAKING] Removed Kubernetes value source

## [0.11.0] - 2023-08-24

### Breaking Changes

- Remove the concept of native commands
  - The `command` value source is effectively gone, and the `shell` value source type has to renamed to `command` to replace the old one
    - In other words, the `shell` type is gone and the `command` field for the `command` type now takes a string instead of a `string[]`
  - Side effects can now only be shell commands (string literals)
  - This is to reduce the overall complexity of the tool. I don't thi
    nk there's a strong use case for native commands, where you can't just use shell commands

### New Features

- Add `cwd` option to `command` value source type, to force the command to execute in a particular directory
- Modifications to the `PATH` variable will be prepended to the existing value, rather than replacing it
  - This special behavior is based on the variable name, and only applies to `PATH`

## [0.10.0] - 2023-08-15

### Breaking Changes

- Cascading config files are now only merged down to the profile level

### New Features

- Added side effects. See usage docs for more. Imperative environment configuration!

### Other

- Sourceable output from `es set` is now written to a temporary file instead of stdout. This difference is handled by the shell functions, so no change to behavior for users

## [0.9.0] - 2023-07-31

### Breaking Changes

- `es show` is now broken into sub-subcommands: `es show config` and `es show shell`
- Unknown keys in config will now be rejected

### New Features

- Add `--run-in-shell` flag to `es run`
- `es run` and `es set` no longer require an application name in the command; if not given, they will prompt, the same way they prompt for profile name

### Other

- Provide more context if the subprocess in `es run` fails

## [0.8.0] - 2023-07-18

### New Features

- Load multiple values from a single source with the `multiple` flag
  - Supported for all value source types
- `file` value source, which loads value(s) from a file path (combine with `multiple = true` for maximum fun!)
- Support non-string primitives for simple literal values
  - E.g. `VARIABLE1 = 123` or `VARIABLE2 = false`
  - These values will simply be stringified before export, since shells only understand strings anyway

### Other

- Improve test coverage!

## [0.7.0] - 2023-07-13

This should be the last release with major breaking changes. The config layout has changed dramatically in order to support planned (and unplanned) future features.

### Breaking Changes

- Removed `vars` config section. You can no longer provide mappings for single variables. Instead, define a set of profiles with single variables
  - This feature didn't provide any additional functionality, it was just a slight convenience at the cost of complexity both for users and code
- Restructured profile config:
  - Renamed `apps` field to `applications`
  - Add new `profiles` and `variables` subfields
  - Overall, this means `apps.app1.profile1.VARIABLE1` will now be `applications.app1.profiles.profile1.variables.VARIABLE1`
  - This is more tedious, but allows for current and future features to fit into the config

### New Features

- Profile inheritance - profiles can now extend other profiles, eliminating the need to copy-paste a bunch of common content between profiles

### Other

- `es` shell function definitions now use the full path to the `env-select` binary rather than relying on `PATH`
  - This eliminates the need to add `env-select` to the `PATH`, and also guarantees that the copy of `env-select` that is being executed by `es` is the one that generated that `es` definition in the first place

## [0.6.2] - 2023-07-08

### Other

- Fix binaries being built for the wrong architecture

## [0.6.1] - 2023-07-08

### Other

- Defer shell path loading until it's needed. This will enable env-select init on systems that don't have the specified shell present

## [0.6.0] - 2023-07-08

I tried to fit all the foreseeable breaking changes into this release, there may be some more though.

### Breaking Changes

- Complex value sources (i.e. anything other than a simple string) now require the `type` field. E.g. `type = "literal"` or `type = "command"`
  - As value sources get more intricate, options start to collide. This field makes it easy to disambiguate between source types, which allows them to have overlapping option names
- Rename `--shell-path` option _back_ to `--shell`, and it once again only requires a shell name, rather than a full path
  - The full path for the shell will be grabbed via the `which` command now. This means whatever shell you use must be in your `PATH`
- Rename `command` value source type to `shell`
  - The old `command` name is now used for commands that are executed natively

### New Features

- Add `run` subcommand, for one-off environment usage
  - This runs a single command in the configured environment, rather than modifying the shell environment. Similar to `kubectl exec` or `poetry run`
- Add `command` value source type, which accepts an array of strings and executes a command natively, rather than via the shell
- Add `kubernetes` value source type, which executes a command in a kubernetes pod via `kubectl`
- Support complex literal values, enabling the `sensitive` option for literals
  - This option probably isn't that useful, but now the field is supported globally
- Add a third level of verbosity (`-vvv`) to enable more granularity in log filtering

### Other

- Fix macOS x86 build in CI (the binary will appear on releases now)
- Add a bunch of tests

## [0.5.0] - 2023-06-30

### New Features

- Shell configuration can now be loaded from `env-select init` function. Add this to your shell startup script to load it automatically. See installation instructions for more info.
- `--shell-path` option allows you to override the `$SHELL` variable. This is rarely necessary, mostly useful for debugging.
- Print configured variables to stderr to give some feedback when running `es set`
- Add `sensitive` option to `command` value source, to mask data in information output
- Support additional verbosity level with `-vv`

### Other

- Dynamic commands are now executed within the scope of env-select . Env-select will run your shell as a subprocess to execute the command, rather than print out a templated string (e.g. `$(echo def)`) to invoke a subshell. This reduces the surface area for bugs, and opens up options new kinds of dynamic values.

## [0.4.1] - 2023-06-25

### Other

Fixed release process. Binaries for 0.4.0 are attached to this release

## [0.4.0] - 2023-06-25

### Breaking Changes

- Renamed binary from `es` to `env-select` (to facilitate shell plugins using `es`)
- Moved main functionality under `env-select set` subcommand

### New Features

- Added fish plugin
- Added `show` subcommand

### Bug Fixes & Tweaks

- Emit non-zero exit code for errors
- Print available variables and applications for bare `env-select set` or an invalid variable/application name

## [0.3.0] - 2023-06-25

### New Features

- Add `command` variant for values, allowing lazily evaluated commands instead of static values

### Other

- Add `aarch64-apple-darwin` to release build
- Upgrade to rust 1.67.1

## [0.2.0] - 2022-11-12

- Reorient schema around named profiles (breaking change)
- Fix terminal cursor disappearing after ctrl-c (#5)
- Allow passing profile name (or literal value, for single variables) as a cmd arg to skip interactive prompt (#3)
- Give profiles/variables consistent ordering in prompt (#2)
- Clean up error handling a bit
- Lots of doc improvements
