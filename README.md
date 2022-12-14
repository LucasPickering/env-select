# env-select

![license](https://img.shields.io/github/license/LucasPickering/env-select)
[![crates.io version](https://img.shields.io/crates/v/env-select.svg)](https://crates.io/crates/env-select)

Easily switch between predefined values for arbitrary environment variables Features include (but are not limited to):

- Interative prompts to select between variable profiles
- Cascading config system, allowing for system and repo-level value definitions

## Simple Example

```toml
# .env-select.toml
[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

```sh
> es server | source
❯ === dev ===
SERVICE1=dev
SERVICE2=also-dev

  === prd ===
SERVICE1=prd
SERVICE2=also-prd
> echo $SERVICE1 $SERVICE2
dev also-dev
```

(See [the disclaimer](#source-disclaimer) for why piping to `source` is needed)

## Concepts

env-select operates with a few different buildings blocks:

- Variable Mapping
- Profile
- Application

### Variable Mapping

A variable mapping an environment variable with a value. A key and a value. Variables can either be selected independently (via the `vars` key in the config) or be part of a profile with other variables.

```sh
SERVICE1=dev
```

### Profile

A profile is a set of variable mappings.

```sh
SERVICE1=dev
SERVICE2=also-dev
```

### Application

An application is a group. "Application" in this case is a synonym for "use case" or "purpose". Each profile in an application accomplishes different versions of the same goal.

```sh
# dev
SERVICE1=dev
SERVICE2=also-dev

# prd
SERVICE1=prd
SERVICE2=also-prd
```

## Usage

First, define `.env-select.toml`. This is where you'll specify possible options for each variable. Here's an example:

```toml
[vars]
TEST_VARIABLE = ["abc", "def"]

[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

[apps.db]
dev = {DATABASE = "dev", DB_USER = "root",  DB_PASSWORD = "badpw"}
stg = {DATABASE = "stg", DB_USER = "root", DB_PASSWORD = "goodpw"}
prd = {DATABASE = "prd", DB_USER = "root", DB_PASSWORD = "greatpw"}
```

Now, you can easily switch between the defined values with `es`.

### Select a single variable

We can select between multiple values for a single variable, in this case `TEST_VARIABLE`. This is a shorthand for defining an application with multiple single-variable profiles.

```sh
> es TEST_VARIABLE | source
  TEST_VARIABLE=abc
❯ TEST_VARIABLE=def
> echo $TEST_VARIABLE
def
```

### Select a set of variables

In the config above, we've already predefined an application called `server`, which consists of two profiles, `dev` and `prd`. We can select between those profiles by providing the _application_ name.

```sh
> es server | source
❯ === dev ===
SERVICE1=dev
SERVICE2=also-dev

  === prd ===
SERVICE1=prd
SERVICE2=also-prd

> echo $SERVICE1 $SERVICE2
dev also-dev
```

If you know the name of the profile you want to select, you can also skip the prompt by providing it directly to the command:

```sh
> es server dev | source
> echo $SERVICE1 $SERVICE2
dev also-dev
```

## Configuration

Configuration is defined in [TOML](https://toml.io/en/). There are two main tables in the config, each defined by a fixed key:

- Single variables, under the `vars` key
  - Each table entry is a mapping from `VARIABLE_NAME` to a list of possible values
- [Applications](#application), under the `apps` key
  - Sub-tables define each [profiles](#profile)
  - Each profile consists of a mapping of `VARIABLE = "value"`

Let's see this in action:

```toml
# Single variables
[vars]
TEST_VARIABLE = ["abc", "def"]
OTHER_VARIABLE = ["potato", "tomato"]

# Applications
[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

# This application has no profiles, but is still valid to configure
[apps.empty]

# These profiles are big, so we can use full table syntax instead of the
# inline syntax. This is purely stylistic; you can make your inline
# tables as big as your heart desires. See https://toml.io/en/v1.0.0#table
[apps.big.prof1]
VAR1 = "yes"
VAR2 = "yes"
VAR3 = "no"
VAR4 = "no"
VAR5 = "yes"

[apps.big.prof2]
VAR1 = "no"
VAR2 = "no"
VAR3 = "no"
VAR4 = "yes"
VAR5 = "no"
```

### Disjoint Profiles

Profiles within an app can define differing sets of variables, like so:

```toml
[apps.db]
dev = {DATABASE = "dev", DB_USER = "root"}
stg = {DATABASE = "stg", DB_USER = "root", DB_PASSWORD = "goodpw"}
prd = {DATABASE = "prd", DB_USER = "root", DB_PASSWORD = "greatpw"}
```

The `dev` profile excludes the `DB_PASSWORD` variable. Beware though, whenever switch to the dev profile, it will simply not output a value for `DB_PASSWORD`. That means if you're switch from another profile, `DB_PASSWORD` will retain its old value! For this reason, it's generally best to define the same set of values for every profile in an app, and just use empty values as appropriate.

### Cascading configs

On every execution, env-select will scan the current directory for a file called `.env-select.toml` and parse it for a config. In addition to that, it will walk up the directory tree and check each ancestor directory tree for the same file. If multiple files are found, the results will be merged together, with **lower config files having higher precedence**. For example, if we execute `es TEST_VARIABLE` in `~/code/`:

```toml
# ~/code/.env-select.toml
[vars]
TEST_VARIABLE = ["abc", "def"]

[apps.server]
dev = {SERVICE1 = "secret-dev-server", SERVICE2 = "another-secret-dev-server"}
```

```toml
# ~/.env-select.toml
[vars]
TEST_VARIABLE = ["ghi"]
OTHER_VARIABLE = ["potato", "tomato"]

[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

then our resulting config, at execution time, will look like:

```toml
# Note: this config never exists in the file system, only in memory during program execution
[vars]
# Variable lists get appended together
TEST_VARIABLE = ["abc", "def", "ghi"]
OTHER_VARIABLE = ["potato", "tomato"]

[apps.server]
# From ~/code/.env-select.toml (higher precedence)
dev = {SERVICE1 = "secret-dev-server", SERVICE2 = "another-secret-dev-server"}
# From ~/.env-select.toml (no value in ~/code/.env-select.toml)
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

To see where env-select is loading configs from, and how they are being merged together, run the command with the `--verbose` (or `-v`) flag.

## Shell Support

env-select supports the following shells:

- bash
- zsh
- fish

If you use a different shell and would like support for it, please open an issue and I'll see what I can do!

## `source` Disclaimer

env-select runs as a subprocess to your shell (as all commands do), meaning it cannot modify your shell environment. To get around this, env-select will simply output shell commands that you can then pipe to `source` to modify your session.

If you think piping stuff to `source` is dangerous and sPoOky, you're right. But consider the fact that at this point, you've already downloaded and executed a mystery binary on your machine. You should've already done your due diligence.

Alternatively, you can run the command without piping to `source`, and it will simply print out the commands to modify your environment, which you can copy-paste to run manually.

## Bugs/Feedback

If you find a bug or have a feature request, please [open an issue on GitHub](https://github.com/LucasPickering/env-select/issues/new).
