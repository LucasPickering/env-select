# Setting Environment Variables

The primary purpose of env-select is to configure environment variables. The most common way to do this is to provide static values:

```toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}

[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

## Dynamic Values

If your values are not statically known, there are several ways to load dynamic values. Fore more detailed information on each option, see [the API reference](../api/value_source.md).

### Shell Command

You can specify a shell command to generate a value. This allows you to provide values that can change over time, or secrets that you don't want appearing in the file. For example:

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "command", command = "curl https://www.random.org/strings/?format=plain&len=10&num=1&loweralpha=on", sensitive = true}
```

When the `dev` profile is selected for the `db` app, the `DB_PASSWORD` value will be randomly generated from a URL. The `sensitive` field is an _optional_ field that will mask the value in informational logging.

The command is executed in the shell detected by env-select as your default (or the shell passed with `--shell`).

### File

You can also load values from a file:

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

This value source combines well with the `multiple` field to load `.env` files (see next section).

## Multiple Values from a Single Source

If you want to load multiple values from a single source, you can use the `multiple = true` flag. This will tell env-select to expect a mapping of `VARIABLE=value` as output from the value source, with one entry per line. Whitespace lines and anything preceded by a `#` will be ignored (this is the standard `.env` file format).

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

### Filtering Loaded Values

If you want to load only _some_ values from a source, you can filter which are loaded by passing a list of variables to `multiple`. This is useful in scenarios where you dump an entire environment. For example:

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
creds = {type = "command", command = "ssh me@remote printenv", multiple = ["DB_USER", "DB_PASSWORD"]}
```

This will only load the `DB_USER` and `DB_PASSWORD` variables:

```sh
> es run db dev -- printenv
DATABASE=dev
DB_USER=root
DB_PASSWORD=hunter2
```

## Adding to the PATH Variable

If you want to modify the `PATH` variable, typically you just want to add to it, rather than replace it. Because of this, env-select will treat the variable `PATH` specially.

```toml
[applications.server.profiles.dev.variables]
PATH = "~/.bin"
```

```sh
> printenv PATH
/bin:/usr/bin
> es run server dev -- printenv PATH
~/.bin:/bin:/usr/bin
```
