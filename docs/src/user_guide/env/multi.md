# Multiple Values from a Single Source

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

## Filtering Loaded Values

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
