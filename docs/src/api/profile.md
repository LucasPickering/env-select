# Profile

A profile is a collection of variable mappings and side effects. It generally maps to a single environment for a deployed application. Here are some example profiles:

```toml
[applications.server.profiles.variables.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.variables.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

# This application has no profiles, but is still valid to configure
[applications.empty]

# These profiles are big, so we can use full table syntax instead of the
# inline syntax. This is purely stylistic; you can make your inline
# tables as big as your heart desires. See https://toml.io/en/v1.0.0#table
[applications.big.profiles.prof1.variables]
VAR1 = "yes"
VAR2 = "yes"
VAR3 = "no"
VAR4 = "no"
VAR5 = "yes"

[applications.big.profiles.prof2.variables]
VAR1 = "no"
VAR2 = "no"
VAR3 = "no"
VAR4 = "yes"
VAR5 = "no"
```

## Disjoint Profiles

Profiles within an app can define differing sets of variables, like so:

```toml
[applications.db.profiles.dev]
variables = {DATABASE = "dev", DB_USER = "root"}
[applications.db.profiles.stg]
variables = {DATABASE = "stg", DB_USER = "root", DB_PASSWORD = "goodpw"}
[applications.db.profiles.prd]
variables = {DATABASE = "prd", DB_USER = "root", DB_PASSWORD = "greatpw"}
```

The `dev` profile excludes the `DB_PASSWORD` variable. Beware though, whenever you switch to the dev profile, it will simply not output a value for `DB_PASSWORD`. That means if you switch from another profile, `DB_PASSWORD` will retain its old value! For this reason, it's generally best to define the same set of values for every profile in an app, and just use empty values as appropriate.

## Fields

| Field         | Type    | Purpose                                          |
| ------------- | ------- | ------------------------------------------------ |
| `variables`   | `table` | Variable:value mapping to export                 |
| `pre_export`  | `array` | Side effects to run _before_ exporting variables |
| `post_export` | `array` | Side effects to run _after_ exporting variables  |
