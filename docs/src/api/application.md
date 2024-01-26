# Application

An application is the highest level of configuration resource in env-select. An application generally corresponds 1:1 with a deployed service or code repository. An application is essentially just a container for profiles. Here are some example application configurations:

```toml
[applications.server.profiles.variables.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.variables.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

# This application has no profiles, but is still valid to configure
[applications.empty]
```

## Fields

| Field      | Type    | Purpose                                                   |
| ---------- | ------- | --------------------------------------------------------- |
| `profiles` | `table` | Name:profile mapping for all profiles of this application |
