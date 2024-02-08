# Setting Environment Variables

The primary purpose of env-select is to configure environment variables. The most common way to do this is to provide static values:

```toml
[applications.server.profiles.dev]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}

[applications.server.profiles.prd]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
```

Beyond these simple values, there are several ways to customize how values are computed. Read on to learn more.
