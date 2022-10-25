# env-select

![license](https://img.shields.io/github/license/LucasPickering/env-select)
[![crates.io version](https://img.shields.io/crates/v/env-select.svg)](https://crates.io/crates/env-select)

Easily switch between predefined values for arbitrary environment variables.

## Usage

First, define `.env-select.toml`. This is where you'll specify possible options for each variable. Here's an example:

```toml
[variables]
TEST_VARIABLE = ["abc", "def"]

[[varsets.important]]
VAR1 = "dev"
VAR2 = "also dev"

[[varsets.important]]
VAR1 = "prd"
VAR2 = "also prd"
```

Now, you can easily switch between the defined values (or specify an adhoc value) with `es`.

Note: As a subprocess, env-select cannot automatically modify your shell environment. To get around this, env-select will simply output shell commands that you can then pipe to `source` to modify your session.

### Select a single variable

We can select between multiple values for a single variable, in this case `TEST_VARIABLE`.

```sh
> es TEST_VARIABLE | source
❯ TEST_VARIABLE=abc
> echo $TEST_VARIABLE
abc
```

### Select a set of variables

In the config above, we've already predefined a varset called `important`, which consists of two variables. We can select between different values for that varset.

```sh
> es important | source
❯ VAR1=dev
  VAR2=also dev
  VAR1=prd
  VAR2=also prd
> echo $VAR1
dev
> echo $VAR2
also dev
```

### Cascading configs

On every execution, env-select will scan the current directory for a file called `.env-select.toml` and parse it for a config. In addition to that, it will walk up the directory tree and check each ancestor directory tree for the same file. If multiple files are found, the results will be merged together, with lower config files having higher precedence. For example, if we execute `es TEST_VARIABLE` in `~/code/`:

```toml
# ~/code/.env-select.toml
[vars]
TEST_VARIABLE = ["abc", "def"]

[[varsets.important]]
VAR1 = "dev"
VAR2 = "also dev"
```

```toml
# ~/.env-select.toml
[vars]
TEST_VARIABLE = ["ghi"]

[[varsets.important]]
VAR1 = "prd"
VAR2 = "also prd"
VAR3 = "default"
```

then our resulting config, at execution time, will look like:

```toml
# Note: this config never exists in the file system, only in memory during program execution
[vars]
TEST_VARIABLE = ["ghi", "abc", "def"]

[[varsets.important]]
VAR1 = "dev"
VAR2 = "also dev"
VAR3 = "default"
```
