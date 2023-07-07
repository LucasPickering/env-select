# Usage Guide

## Table of Contents

If viewing this [in GitHub](https://github.com/LucasPickering/env-select/blob/master/USAGE.md), use the Outline button in the top-right to view a table of contents.

## Concepts

env-select operates with a few different building blocks. From smallest to largest, they are: Value Source, Variable Mapping, Profile, and Application.

### Value Source

A value source is a means of deriving a string for the shell. Typically this is just a literal string: `"abc"`, but it can also be a command that will be evaluated to a string at runtime.

```sh
dev # Literal
$(echo prd) # Command
```

### Variable Mapping

A key and a value source. Variables can either be selected independently (via the `vars` key in the config) or be part of a profile with other variables.

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
> es set TEST_VARIABLE
  TEST_VARIABLE=abc
❯ TEST_VARIABLE=def
> echo $TEST_VARIABLE
def
```

### Select a set of variables

In the config above, we've already predefined an application called `server`, which consists of two profiles, `dev` and `prd`. We can select between those profiles by providing the _application_ name.

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

If you know the name of the profile you want to select, you can also skip the prompt by providing it directly to the command:

```sh
> es set server dev
> echo $SERVICE1 $SERVICE2
dev also-dev
```

### Run a single command

If you want to run only a single command in the modified environment, rather than modify the entire shell, you can use `es run` instead of `es set`:

```sh
# Select the profile to use for the `server` application, then run the command
> es run server -- echo $SERVICE1 $SERVICE2
❯ === dev ===
SERVICE1=dev
SERVICE2=also-dev

  === prd ===
SERVICE1=prd
SERVICE2=also-prd

dev also-dev
# You can also specify the profile name up front
> es run server dev -- echo $SERVICE1 $SERVICE2
dev also-dev
# The surrounding environment is *not* modified
> echo $SERVICE1 $SERVICE2

```

`--` is required to delineate the arguments handled by `es` from the command being executed. The executed command is called directly, _not_ executed in a shell. To access shell features in the executed command, you can explicitly run in a subshell:

```sh
> es run server dev -- sh -c 'echo $SERVICE1 | cat -'
dev
```

Make sure to use **single** quotes in those case, otherwise `$SERVICE1` will be evaluted by your shell _before_ executing env-select.

### Dynamic Values

You can define variables whose values are provided dynamically, by specifying a command to execute rather than a static value. This allows you to provide values that can change over time, or secrets that you don't want appearing in the file. For example:

```toml
[apps.db.dev]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "command", command = ["cat", "password.txt"], sensitive = true}
```

When the `dev` profile is selected for the `db` app, the `DB_PASSWORD` value will be loaded from the file `password.txt`. The `sensitive` field is an _optional_ field that will mask the value in informational logging.

By default, the program (the first argument in the list) is executed directly by env-select, and passed the rest of the list as arguments. If you want to execute a command in your shell, you can use the `shell` type instead. This will give access to shell features such as aliases and pipes. For example:

```toml
[apps.db.dev]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "shell", command = "echo password | base64", sensitive = true}
```

### Load Values from Kubernetes

Ever had a secret in a Kubernetes pod that you want to fetch easily? The `kubernetes` value source lets you run any command in a kubernetes pod.

```toml
[apps.db.dev]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "kubernetes", namespace = "development", pod_selector = "app=api", command = ["printenv", "DB_PASSWORD"]}

[apps.db.prd]
DATABASE = "prd"
DB_USER = "root"
DB_PASSWORD = {type = "kubernetes", namespace = "production", pod_selector = "app=api", command = ["printenv", "DB_PASSWORD"]}
```

`printenv` can be used to easily access environment variables, but you can execute any command you want in the pod. To access shell features in the pod, you'll need to execute under a shell. For example:

```
command = ["sh", "-c", "env | grep DB_PASSWORD | sed -E 's/.+=(.+)/\1/'"]
```

## Configuration

Configuration is defined in [TOML](https://toml.io/en/). There are two main tables in the config, each defined by a fixed key:

- Single variables, under the `vars` key
  - Each table entry is a mapping from `VARIABLE_NAME` to a list of possible value sources
- [Applications](#application), under the `apps` key
  - Sub-tables define each [profiles](#profile)
  - Each profile consists of a mapping of `VARIABLE = <value source>`

Let's see this in action:

```toml
# Single variables
[vars]
TEST_VARIABLE = ["abc", "def", {value = "secret", sensitive = true}]
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

On every execution, env-select will scan the current directory for a file called `.env-select.toml` and parse it for a config. In addition to that, it will walk up the directory tree and check each ancestor directory tree for the same file. If multiple files are found, the results will be merged together, with **lower config files having higher precedence**. For example, if we execute `es set TEST_VARIABLE` in `~/code/`:

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

## Configuration Reference

### Value Source

There are multiple types of value sources. The type used for a value source is determined by the `type` field. For example:

```toml
# All of these examples will generate the same exported value
[vars]
GREETING = [
  "hello", # Literal - shorthand (most common)
  {type = "literal", value = "hello!"}, # Literal - expanded form
  {type = "command", command = ["echo", "hello!"]}, # Native command
  {type = "shell", command = "echo hello"}, # Shell command
]
```

#### Value Source Types

| Value Source Type | Description                              |
| ----------------- | ---------------------------------------- |
| `literal`         | Literal static value                     |
| `command`         | Execute an external program by name/path |
| `shell`           | Execute a shell command                  |
| `kubernetes`      | Execute a command in a Kubernetes pod    |

#### Common Fields

All value sources support the following common fields:

| Option      | Type      | Default | Description                  |
| ----------- | --------- | ------- | ---------------------------- |
| `sensitive` | `boolean` | `false` | Hide value in console output |

#### Type-Specific Fields

Each source type has its own set of available fields:

| Value Source Type | Field          | Type            | Default      | Description                                                                                                                                                                       |
| ----------------- | -------------- | --------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `literal`         | `value`        | `string`        | **Required** | Static value to export                                                                                                                                                            |
| `command`         | `command`      | `array[string]` | **Required** | Command to execute, as `[program, ...arguments]`; the output of the command will be exported                                                                                      |
| `shell`           | `command`      | `string`        | **Required** | Command to execute in a subshell; the output of the command will be exported                                                                                                      |
| `kubernetes`      | `command`      | `array[string]` | **Required** | Command to execute in the pod, as `[program, ...arguments]`; the output of the command will be exported                                                                           |
| `kubernetes`      | `pod_selector` | `string`        | **Required** | Label query used to find the target pod. Must match exactly one pod. See [kubectl docs](https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/) for more info. |
| `kubernetes`      | `namespace`    | `string`        | `null`       | Namespace in which to search for the target pod. If omitted, `kubectl` will use the current context namespace.                                                                    |
| `kubernetes`      | `container`    | `string`        | `null`       | Container within the target pod to execute in. If omitted, `kubectl` will use the default defined by the `kubectl.kubernetes.io/default-container` annotation.                    |

## Shell Support

env-select supports the following shells:

- bash
- zsh
- fish

If you use a different shell and would like support for it, please open an issue and I'll see what I can do!
