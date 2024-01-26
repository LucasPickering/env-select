# Getting Started

## Create the configuration file

Once you've [installed env-select](/artifacts), setup is easy. All configuration is defined in the [TOML](https://toml.io/en/) format. Create a file called `.env-select.toml` to hold your configuration. This file will apply to to your current directory **and all descendent directories**. In other words, any time you run the `es` command, it will search _up_ the directory tree the `.env-select.toml` file.

Here's an example `.env-select.toml` file:

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

## Select a set of variables

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

## Run a single command

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

`--` is required to delineate the arguments handled by `es` from the command being executed. The executed command is executed in your shell, so you can access shell features such as pipes and aliases.
