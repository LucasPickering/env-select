# Side Effects

Side effects allow you to configure your environment beyond simple environment variables, using imperative commands. Each side effects has two commands: setup and teardown. Additionally, there are two points at which side effects can execute: pre-export (before environment variables are exported) and post-export (with environment variables available). So there are four side effect stages in total (in their order of execution):

- Pre-export setup
- Post-export setup
- Post-export teardown
- Pre-export teardown

The meaning of "setup" and "teardown" varies based on what subcommand you're running: `es set` has no teardown stage, as its purpose is to leave the configured environment in place. Currently there is no way to tear down an `es set` environment (see [#37](https://github.com/LucasPickering/env-select/issues/37)). For `es run`, setup occurs before executing the given command, and teardown occurs after.

While supplying both setup and teardown commands isn't required, it's best practice to revert whatever changes your setup command may have made. You should only omit the teardown function if your setup doesn't leave any lingering changes in the environment.

## Examples

Given this config:

```toml
[applications.server.profiles.base]
# These commands *cannot* access the constructed environment
pre_export = [
  # Native commands - not executed through the shell
  {setup = ["touch", "host.txt"], teardown = ["rm", "-f", "host.txt"]}
]
# These commands can use the constructed environment
post_export = [
  # Shell command - no teardown needed because the above command handles it
  {setup = "echo https://$SERVICE1 > host.txt"}
]


[applications.server.profiles.dev]
extends = ["base"]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}

[applications.server.profiles.prd]
extends = ["base"]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

This will execute in the followingn order for `es set`:

```sh
> es set server dev
# 1. Execute pre-export setup (host.txt is created)
# 2. Construct environment
# 3. Execute post-export setup (host URL is written to host.txt)
# 4. Environment is exported to your shell
> echo $SERVICE1
dev
> cat host.txt
https://dev
```

And for `es run`:

```sh
> es run server dev -- cat host.txt
# 1. Execute pre-export setup (host.txt is created)
# 2. Construct environment
# 3. Execute post-export setup (host URL is written to host.txt)
# 4. `cat host.txt`
https://dev
# 5. Execute post-export teardown (in this case, nothing)
# 6. Clear constructed environment variables
# 7. Execute pre-export teardown (host.txt is deleted)
> cat host.txt
cat: host.txt: No such file or directory
```

## Ordering

Side effects are executed in their order of definition for setup, and the **reverse** order for teardown. This is to enable side effects that depend on each other; the dependents are torn down before the parents are.

## Inheritance

Inherited side effects are executed _before_ side effects defined in the selected profile during setup, and therefore _after_ during teardown. For profiles with multiple parents, the _left-most_ parent's side effects will execute first.

An example of a config with inheritance:

```toml
[applications.server.profiles.base1]
pre_export = [{setup = "echo base1 setup", teardown = "echo base1 teardown"}]

[applications.server.profiles.base2]
pre_export = [{setup = "echo base2 setup", teardown = "echo base2 teardown"}]

[applications.server.profiles.child]
extends = ["base1", "base2"]
pre_export = [{setup = "echo child setup", teardown = "echo child teardown"}]
```

And how the inheritance would resolve for the `child` profile:

```toml
[applications.server.profiles.child]
pre_export = [
  {setup = "echo base1 setup", teardown = "echo base1 teardown"},
  {setup = "echo base2 setup", teardown = "echo base2 teardown"},
  {setup = "echo child setup", teardown = "echo child teardown"},
]
```

Here's the order of command execution:

```sh
> es run server child -- echo hello
base1 setup
base2 setup
child setup
hello
child teardown
base2 teardown
base1 teardown
```
