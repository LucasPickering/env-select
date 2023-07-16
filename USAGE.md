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

A key mapped to a value source. Variables are selected as part of a profile.

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
[applications.server.profiles.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}

[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

[applications.db.profiles.dev]
variables = {DATABASE = "dev", DB_USER = "root",  DB_PASSWORD = "badpw"}
[applications.db.profiles.stg]
variables = {DATABASE = "stg", DB_USER = "root", DB_PASSWORD = "goodpw"}
[applications.db.profiles.prd]
variables = {DATABASE = "prd", DB_USER = "root", DB_PASSWORD = "greatpw"}
```

Now, you can easily switch between the defined values with `es`.

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

If you know the name of the profile you want to select, you can skip the prompt by providing it directly to the command:

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
[applications.db.profiles.dev.variables]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "command", command = ["curl", "https://www.random.org/strings/?format=plain&len=10&num=1&loweralpha=on"], sensitive = true}
```

When the `dev` profile is selected for the `db` app, the `DB_PASSWORD` value will be loaded from the file `password.txt`. The `sensitive` field is an _optional_ field that will mask the value in informational logging.

By default, the program (the first argument in the list) is executed directly by env-select, and passed the rest of the list as arguments. If you want to execute a command in your shell, you can use the `shell` type instead. This will give access to shell features such as aliases and pipes. For example:

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "shell", command = "echo password | base64", sensitive = true}
```

### Multiple Values from a Single Source

If you want to load multiple values from a single source, you can use the `multiple = true` flag. This will tell env-select to expect a mapping of `VARIABLE=value` as output from the value source, with one entry per line. Whitespace lines and anything preceded by a `#` will be ignored (this isthe standard `.env` file format).

This means that **the key associated with this entry in the `variables` map will be ignored**.

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
creds = {type = "file", path = "creds.env", multiple = true}
```

`creds.env`:

```sh
DB_USER=root
DB_PASSWORD=hunter2
```

`creds` will now be expanded into multiple variables:

```sh
> es run db dev -- printenv
DATABASE=dev
DB_USER=root
DB_PASSWORD=hunter2
```

Notice the `creds` key never appears in the environment; this is just a placeholder. You can use any key you want here.

### Load Values from a File

You can load one values from a file.

```toml
[applications.db.profiles.dev.variables]
DATABASE = {type = "file", path = "database.txt"}
```

`database.txt`:

```
dev
```

```sh
> es run db dev -- printenv
DATABASE=dev
```

The file path will be relative to **the config file in which the path is defined**. For example, if you have two `.env-select.toml` files:

```toml
# ~/code/.env-select.toml
[applications.server.profiles.dev.variables]
SERVICE1 = {type = "file", path = "service1.txt"}
```

```toml
# ~/.env-select.toml
[applications.server.profiles.dev.variables]
SERVICE2 = {type = "file", path = "service2.txt"}
```

In this scenario, `SERVICE1` will be loaded from `~/code/service1.txt` while `SERVICE2` will be loaded from `~/service2.txt`, **regardless of where env-select is invoked from**.

This value source combines well with the `multiple` field to load `.env` files. [See here](#multiple-values-from-a-single-source).

### Load Values from Kubernetes

Ever had a secret in a Kubernetes pod that you want to fetch easily? The `kubernetes` value source lets you run any command in a kubernetes pod.

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "kubernetes", namespace = "development", pod_selector = "app=api", command = ["printenv", "DB_PASSWORD"]}

[applications.db.profiles.prd.variables]
DATABASE = "prd"
DB_USER = "root"
DB_PASSWORD = {type = "kubernetes", namespace = "production", pod_selector = "app=api", command = ["printenv", "DB_PASSWORD"]}
```

`printenv` can be used to easily access environment variables, but you can execute any command you want in the pod. To access shell features in the pod, you'll need to execute under a shell. For example:

```
command = ["sh", "-c", "env | grep DB_PASSWORD | sed -E 's/.+=(.+)/\1/'"]
```

You can combine this with the `multiple = true` flag to fetch multiple values at once:

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
# The `creds` key is *not* significant here - you can name it anything you want
[applications.db.profiles.dev.variables.creds]
type = "kubernetes"
namespace = "development"
pod_selector = "app=api"
command = ["sh", "-c", "printenv | grep -E '^(DB_USER|DB_PASSWORD)='"]
multiple = true
```

## Configuration

Configuration is defined in [TOML](https://toml.io/en/). Let's see this in action:

```toml
[applications.server.profiles.variables.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.variables.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

# This application has no profiles, but is still valid to configure
[applications.empty]

# These profiles are big, so we can use full table syntax instead of the
# inline syntax. This is purely stylistic; you can make your inline
# tables as big as your heart desires. See https://toml.io/en/v1.0.0#table
[applications.big.profiles.prof1.variables]
VAR1 = "yes"
VAR2 = "yes"
VAR3 = "no"
VAR4 = "no"
VAR5 = "yes"

[applications.big.profiles.prof2.variables]
VAR1 = "no"
VAR2 = "no"
VAR3 = "no"
VAR4 = "yes"
VAR5 = "no"
```

### Disjoint Profiles

Profiles within an app can define differing sets of variables, like so:

```toml
[applications.db.profiles.dev]
variables = {DATABASE = "dev", DB_USER = "root"}
[applications.db.profiles.stg]
variables = {DATABASE = "stg", DB_USER = "root", DB_PASSWORD = "goodpw"}
[applications.db.profiles.prd]
variables = {DATABASE = "prd", DB_USER = "root", DB_PASSWORD = "greatpw"}
```

The `dev` profile excludes the `DB_PASSWORD` variable. Beware though, whenever switch to the dev profile, it will simply not output a value for `DB_PASSWORD`. That means if you're switch from another profile, `DB_PASSWORD` will retain its old value! For this reason, it's generally best to define the same set of values for every profile in an app, and just use empty values as appropriate.

### Cascading configs

On every execution, env-select will scan the current directory for a file called `.env-select.toml` and parse it for a config. In addition to that, it will walk up the directory tree and check each ancestor directory tree for the same file. If multiple files are found, the results will be merged together, with **lower config files having higher precedence**. For example, if we execute `es set SERVICE1` in `~/code/`:

```toml
# ~/code/.env-select.toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "secret-dev-server", SERVICE2 = "another-secret-dev-server"}
```

```toml
# ~/.env-select.toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

then our resulting config, at execution time, will look like:

```toml
# Note: this config never exists in the file system, only in memory during program execution
# From ~/code/.env-select.toml (higher precedence)
[applications.server.profiles.dev]
variables = {SERVICE1 = "secret-dev-server", SERVICE2 = "another-secret-dev-server"}
# From ~/.env-select.toml (no value in ~/code/.env-select.toml)
[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

To see where env-select is loading configs from, and how they are being merged together, run the command with the `--verbose` (or `-v`) flag.

## Profile Inheritance

In addition to top-level merging of multiple config files, env-select also supports inheritance between profiles, via the `extends` field on a profile. For example:

```toml
[applications.server.profiles.base]
variables = {PROTOCOL = "https"}
[applications.server.profiles.dev]
extends = ["base"]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.prd]
extends = ["base"]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

During execution, env-select will merge each profile with its parent(s):

```sh
> es set server
❯ === base ===
PROTOCOL=https

  === dev ===
SERVICE1=dev
SERVICE2=also-dev
PROTOCOL=https

  === prd ===
SERVICE1=prd
SERVICE2=also-prd
PROTOCOL=https
```

Note the `PROTOCOL` variable is available in `dev` and `prd`. The profile name given in `extends` is assumed to be a profile of the same application. To extend a profile from another application, use the format `application/profile`:

```toml
[applications.common.profiles.base]
variables = {PROTOCOL = "https"}
[applications.server.profiles.dev]
extends = ["common/base"]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.prd]
extends = ["common/base"]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

#### Multiple Inheritance and Precedence

Each profile can extend multiple parents. If two parents have conflicting values, the **left-most** parent has precedence:

```toml
[applications.server.profiles.base1]
variables = {PROTOCOL = "https"}
[applications.server.profiles.base2]
variables = {PROTOCOL = "http"}
[applications.server.profiles.dev]
extends = ["base1", "base2"]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
```

The value from `base1` is used:

```sh
> es run server dev -- printenv PROTOCOL
https
```

Inheritance is applied recursively, meaning you can have arbitrarily large inheritance trees, **as long as there are no cycles**.

## Configuration Reference

### Value Source

There are multiple types of value sources. The type used for a value source is determined by the `type` field. For example:

```toml
# All of these profiles will generate the same exported value for GREETING

# Literal shorthand - most common
[applications.example.profiles.shorthand.variables]
GREETING = "hello"

# Literal expanded form - generally not needed
[applications.example.profiles.literal.variables]
GREETING = {type = "literal", value = "hello"},

[applications.example.profiles.command.variables]
GREETING = {type = "command", command = ["echo", "hello"]}, # Native command

[applications.example.profiles.shell.variables]
GREETING = {type = "shell", command = "echo hello | cat -"}, # Shell command
```

#### Value Source Types

| Value Source Type | Description                              |
| ----------------- | ---------------------------------------- |
| `literal`         | Literal static value                     |
| `file`            | Load values from a file                  |
| `command`         | Execute an external program by name/path |
| `shell`           | Execute a shell command                  |
| `kubernetes`      | Execute a command in a Kubernetes pod    |

#### Common Fields

All value sources support the following common fields:

| Option      | Type      | Default | Description                                                                                                   |
| ----------- | --------- | ------- | ------------------------------------------------------------------------------------------------------------- |
| `multiple`  | `boolean` | `false` | Load a `VARIABLE=value` mapping, instead of just a `value`; [see more](#multiple-values-from-a-single-source) |
| `sensitive` | `boolean` | `false` | Hide value in console output                                                                                  |

#### Type-Specific Fields

Each source type has its own set of available fields:

| Value Source Type | Field          | Type            | Default      | Description                                                                                                                                                                       |
| ----------------- | -------------- | --------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `literal`         | `value`        | `string`        | **Required** | Static value to export                                                                                                                                                            |
| `file`            | `path`         | `string`        | **Required** | Path to the file, relative to **the config file in which this is defined**                                                                                                        |
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

```

```
