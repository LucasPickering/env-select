# Adding to the PATH Variable

If you want to modify the `PATH` variable, typically you just want to add to it, rather than replace it. Because of this, env-select will treat the variable `PATH` specially. It will append to the beginning, using `:` as a separator:

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
