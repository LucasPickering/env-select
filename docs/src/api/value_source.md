# Value Source

There are multiple types of value sources. The type used for a value source is determined by the `type` field. For example:

```toml
# All of these profiles will generate the same exported value for GREETING

# Literal shorthand - most common
[applications.example.profiles.shorthand.variables]
GREETING = "hello"

# Literal expanded form - generally not needed
[applications.example.profiles.literal.variables]
GREETING = {type = "literal", value = "hello"}

[applications.example.profiles.command.variables]
GREETING = {type = "command", command = "echo hello"}
```

## Value Source Types

| Value Source Type | Description             |
| ----------------- | ----------------------- |
| `literal`         | Literal static value    |
| `file`            | Load values from a file |
| `command`         | Execute a shell command |

## Common Fields

All value sources support the following common fields:

| Option      | Type      | Default | Description                                                                                                   |
| ----------- | --------- | ------- | ------------------------------------------------------------------------------------------------------------- |
| `multiple`  | `boolean` | `false` | Load a `VARIABLE=value` mapping, instead of just a `value`; [see more](#multiple-values-from-a-single-source) |
| `sensitive` | `boolean` | `false` | Hide value in console output                                                                                  |

## Type-Specific Fields

Each source type has its own set of available fields:

| Value Source Type | Field     | Type     | Default      | Description                                                                                                                                                                                 |
| ----------------- | --------- | -------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `literal`         | `value`   | `string` | **Required** | Static value to export                                                                                                                                                                      |
| `file`            | `path`    | `string` | **Required** | Path to the file, relative to **the config file in which this is defined**                                                                                                                  |
| `command`         | `command` | `string` | **Required** | Command to execute in a subshell; the output of the command will be exported                                                                                                                |
| `command`         | `cwd`     | `string` | `null`       | Directory from which to execute the command. Defaults to the directory from which `es` was invoked. Paths will be relative to the `.env-select.toml` file in which this command is defined. |
