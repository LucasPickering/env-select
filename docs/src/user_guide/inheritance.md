# Inheritance & Cascading Configs

env-select supports two different features that enable sharing configuration: cascading configs and profile inheritance. Cascading configs automatically combines multiple `.env-select.toml` files into one config, while profile inheritance allows you to explicitly re-use variable mappings and side effects from other profiles.

## Cascading configs

On every execution, env-select will scan the current directory for a file called `.env-select.toml` and parse it for a config. In addition to that, it will walk up the directory tree and check each ancestor directory tree for the same file. If multiple files are found, the results will be merged together, **down to the profile level only**. Lower config files having higher precedence. For example, if we execute `es set SERVICE1` in `~/code/`:

```toml
# ~/code/.env-select.toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "secret-dev-server", SERVICE2 = "another-secret-dev-server"}
[applications.server.profiles.stg]
variables = {SERVICE1 = "secret-stg-server", SERVICE2 = "another-secret-stg-server"}
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
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

# From ~/.env-select.toml (no value in ~/code/.env-select.toml)
[applications.server.profiles.stg]
variables = {SERVICE1 = "secret-stg-server", SERVICE2 = "another-secret-stg-server"}
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

The profile name given in `extends` is assumed to be a profile of the same application. To extend a profile from another application, use the format `application/profile`:

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

### Multiple Inheritance and Precedence

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
