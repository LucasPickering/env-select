# Key Concepts

env-select operates with a few different building blocks. From largest to smallest (rougly), they are: [Application](../api/application.md), [Profile](../api/profile.md), Variable Mapping, [Value Source](../api/value_source.md) and Side Effect.

## Application

An application is a group. "Application" in this case is a synonym for "use case" or "purpose". Each profile in an application accomplishes different versions of the same goal. Applications tend to map one-to-one to services or code repositories, but don't necessarily have to.

```sh
# dev
SERVICE1=dev
SERVICE2=also-dev

# prd
SERVICE1=prd
SERVICE2=also-prd
```

See the [API reference](../api/application.md) for more.

## Profile

A profile is a set of variable mappings.

```sh
SERVICE1=dev
SERVICE2=also-dev
```

See the [API reference](../api/profile.md) for more.

## Variable Mapping

A key mapped to a value source. Variables are selected as part of a profile.

```sh
SERVICE1=dev
```

## Value Source

A value source is a means of deriving a string for the shell. Typically this is just a literal string: `"abc"`, but it can also be a command that will be evaluated to a string at runtime.

```sh
dev # Literal
$(echo prd) # Command
```

See the [API reference](../api/value_source.md) for more.

## Side Effect

A side effect is a pairing of procedures: one to execute during environment, and one during teardown. These are used to perform environment configuration beyond environment variables. An example of a side effect is creating a file during setup, then deleting it during teardown.

See the [user guide](./side_effects.md) for more.
