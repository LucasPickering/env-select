# Introduction

env-select is a command line tool that makes it easy to define reusable shell environments. Its most common use case (but not only) is to manage deployed environments when working with webapps. For example, if you have a webapp that has local, staging, and production environments, you can use env-select to set environment variables corresponding to each environment:

```toml
[applications.my_webapp.profiles.local.variables]
PROTOCOL = "http"
HOST = "localhost"
PORT = 3000

[applications.my_webapp.profiles.staging.variables]
PROTOCOL = "https"
HOST = "staging.my.webapp"
PORT = 443

[applications.my_webapp.profiles.production.variables]
PROTOCOL = "https"
HOST = "production.my.webapp"
PORT = 443
```

env-select integrates with your shell to make it easy to configure environments. There are two possible ways to use env-select's environments:

- `es set` - Export values to modify your current shell
- `es run` - Run a single command under the environment, without modifying your shell

Read on to [Getting Started](./getting_started.md) to learn how to use env-select!
