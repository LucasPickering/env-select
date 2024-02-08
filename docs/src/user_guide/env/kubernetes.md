# Load Values from Kubernetes

If you want to load one or more values from a Kubernetes pod, you can do that with the `command` value source:

```toml
[applications.my-service.profile.dev]
variables.DB_PASSWORD = {type = "command", sensitive = true, command = "kubectl exec -n development api -- printenv DB_PASSWORD"}
```

Loading multiple variables is easy too:

```toml
[applications.my-service.profile.dev]
variables.db_creds = {type = "command", sensitive = true, multiple = ["DB_USERNAME", "DB_PASSWORD"], command = "kubectl exec -n development api -- printenv"}
```
