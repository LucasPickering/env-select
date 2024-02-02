# env-select

![license](https://img.shields.io/github/license/LucasPickering/env-select)
[![crates.io version](https://img.shields.io/crates/v/env-select.svg)](https://crates.io/crates/env-select)

- [Home Page](https://env-select.lucaspickering.me)
- [Installation](https://env-select.lucaspickering.me/artifacts/)
- [Docs](https://env-select.lucaspickering.me/book/)
- [Changelog](https://env-select.lucaspickering.me/changelog/)

Easily switch between predefined values for arbitrary environment variables Features include (but are not limited to):

- Interative prompts to select between variable profiles
- Cascading config system, allowing for system and repo-level value definitions
- Grab values dynamically via shell commands
- Modify your shell environment with `es set`, or run a one-off command in a modified environment with `es run`
- Re-use common variables between profiles with inheritance

## Example

```toml
# .env-select.toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}

[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

Now pick an environment to export:

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

## `source` Disclaimer

env-select runs as a subprocess to your shell (as all commands do), meaning it cannot modify your shell environment. To get around this, env-select will simply output shell commands that the shell plugins (or you) can then pipe to `source` (or `eval`) to modify your session.

If you think piping stuff to `source` is dangerous and sPoOky, you're right. But consider the fact that at this point, you've already downloaded and executed a mystery binary on your machine. You should've already done your due diligence.

## Bugs/Feedback

If you find a bug or have a feature request, please [open an issue on GitHub](https://github.com/LucasPickering/env-select/issues/new).
