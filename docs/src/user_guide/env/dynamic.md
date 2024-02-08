# Dynamic Values

If your values are not statically known, there are several ways to load dynamic values. Fore more detailed information on each option, see [the API reference](../api/value_source.md).

## Shell Command

You can specify a shell command to generate a value. This allows you to provide values that can change over time, or secrets that you don't want appearing in the file. For example:

```toml
[applications.db.profiles.dev.variables]
DATABASE = "dev"
DB_USER = "root"
DB_PASSWORD = {type = "command", command = "curl https://www.random.org/strings/?format=plain&len=10&num=1&loweralpha=on", sensitive = true}
```

When the `dev` profile is selected for the `db` app, the `DB_PASSWORD` value will be randomly generated from a URL. The `sensitive` field is an _optional_ field that will mask the value in informational logging.

The command is executed in the shell detected by env-select as your default (or the shell passed with `--shell`).

## File

You can also load values from a file:

```toml
[applications.db.profiles.dev.variables]
DATABASE = {type = "file", path = "database.txt"}
```

`database.txt`:

```
dev
```

And now run it:

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
