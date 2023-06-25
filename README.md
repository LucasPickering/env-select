# env-select

![license](https://img.shields.io/github/license/LucasPickering/env-select)
[![crates.io version](https://img.shields.io/crates/v/env-select.svg)](https://crates.io/crates/env-select)

Easily switch between predefined values for arbitrary environment variables Features include (but are not limited to):

- Interative prompts to select between variable profiles
- Cascading config system, allowing for system and repo-level value definitions

## Table of Contents

- [Installation](#installation)
- [Usage Guide](USAGE.md)
- [Disclaimer](#source-disclaimer)
- [Bugs/Feedback](#bugsfeedback)

## Simple Example

```toml
# .env-select.toml
[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

```sh
> es set server
❯ === dev ===
SERVICE1=dev
SERVICE2=also-dev

  === prd ===
SERVICE1=prd
SERVICE2=also-prd
> echo $SERVICE1 $SERVICE2
dev also-dev
```

## Installation

env-select has two components: the main binary and the shell plugins. Currently the binary can only be installed via `cargo`:

```sh
cargo install env-select
```

The shell plugins are not required, but make usage easier. Otherwise, you have to manually pipe the output of each `env-select` invocation to `source`.

**All commands in this README assume you have the appropriate shell plugin installed.** If you don't replace any command `es ...` with `env-select ... | source`. See [the disclaimer](#source-disclaimer) for why piping to `source` is needed.

### Fish

The easiest way to install is with [fisher](https://github.com/jorgebucaran/fisher).

```sh
fisher install LucasPickering/env-select
```

Or install manually:

```sh
curl https://raw.githubusercontent.com/LucasPickering/env-select/master/functions/es.fish -o ~/.config/fish/functions/es.fish
```

### Bash/Zsh

Coming Soon™

## `source` Disclaimer

env-select runs as a subprocess to your shell (as all commands do), meaning it cannot modify your shell environment. To get around this, env-select will simply output shell commands that the shell plugins (or you) can then pipe to `source` to modify your session.

If you think piping stuff to `source` is dangerous and sPoOky, you're right. But consider the fact that at this point, you've already downloaded and executed a mystery binary on your machine. You should've already done your due diligence.

Alternatively, you can run `env-select` without piping to `source`, and it will simply print out the commands to modify your environment, which you can copy-paste to run manually.

## Bugs/Feedback

If you find a bug or have a feature request, please [open an issue on GitHub](https://github.com/LucasPickering/env-select/issues/new).
