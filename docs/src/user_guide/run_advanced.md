# `es run` and Shell Interactions

You may want to use `es run` in combination with shell features such as quotes, variable expansion, and command substitution. The general rule with `es run` is:

> **The passed command will be executed exactly as the same as if `es run` weren't there, except with a different set of environment variables.**

Here's a concrete example:

```toml
# We'll use this profile for all examples
[applications.server.profiles.dev]
variables.SERVICE1 = "dev"
variables.SERVICE2 = "also-dev"
```

```sh
# These two invocations of `echo` will look exactly the same to the shell
es run server dev -- echo 'hi'
echo 'hi'
```

These two commands will behave exactly the same. In other words, if you're not sure how your command will be executed, ignore everything up to and including the `--`, and that's what the shell will see.

This allows you to add `es run` to any existing shell command without having to mess around with quotes and backslashes. Here's a more complex example where it's handy:

```sh
# Note: this is how you escape single quotes in fish. This command may look
# slightly different in other shells.
es run server dev -- psql -c 'select id from products where cost = \'$30\';'
```

This will pass the exact string `select id from products where cost = '$30';` to `psql`, without any variable expansion or other shell tomfoolery.

> If you encounter any scenarios where the executed command does _not_ behave the same as passing it directly to the shell, please [file a bug](https://github.com/LucasPickering/env-select/issues/new).

## Intentional Variable Expansion

By default, there are two ways to handle variable expansion with `es run`: before invoking `es`, and not at all:

```sh
es run server dev -- echo $SERVICE1 # prints ""
es run server dev -- echo '$SERVICE1' # prints "$SERVICE1"
```

In the first example, `$SERVICE1` is expanded by the shell _before_ `es` is executed. In the second example, the single quotes prevent the shell from ever expanding `$SERVICE1`.

If you _want_ a variable to be expanded after exporting the profile before executing the command, e.g. if you want to use a variable defined by your profile, you'll need to manually invoke a subshell:

```sh
es run server dev -- fish -c 'echo $SERVICE1' # prints "dev"
```
