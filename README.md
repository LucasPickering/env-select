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

See the [Usage Guide](USAGE.md) for more detailed examples.

## Installation

Currently, env-select can only be installed via `cargo`:

```sh
cargo install env-select
```

### Configure Your Shell

Because env-select modifies your shell environment, it requires a wrapper function defined in the shell that can call the `env-select` binary and automatically apply its output.

**All commands in this README assume you have the appropriate shell configuration.** See [the disclaimer](#source-disclaimer) for why piping to `source` is needed.

#### Bash

```sh
echo 'eval "$(env-select init)"' >> ~/.bashrc
```

#### Zsh

```sh
echo 'source <(env-select init)' >> ~/.zshrc
```

#### Fish

```sh
echo 'env-select init | source' >> ~/.config/fish/config.fish
```

**Restart your shell (or `source <file>`) after running the above command.**

## `source` Disclaimer

env-select runs as a subprocess to your shell (as all commands do), meaning it cannot modify your shell environment. To get around this, env-select will simply output shell commands that the shell plugins (or you) can then pipe to `source` (or `eval`) to modify your session.

If you think piping stuff to `source` is dangerous and sPoOky, you're right. But consider the fact that at this point, you've already downloaded and executed a mystery binary on your machine. You should've already done your due diligence.

## Bugs/Feedback

If you find a bug or have a feature request, please [open an issue on GitHub](https://github.com/LucasPickering/env-select/issues/new).
