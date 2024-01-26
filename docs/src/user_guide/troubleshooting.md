# Troubleshooting

## `es: command not found`

Because env-select modifies your shell environment, it requires a wrapper function defined in the shell that can call the `env-select` binary and automatically apply its output.

This error indicates the `es` shell function has not been loaded. Generally it should be installed by the installer, but depending on what shell you use and how you installed env-select, it may be missing. If so, follow the steps for your shell:

#### Bash

```sh
echo 'eval "$(env-select --shell bash init)"' >> ~/.bashrc
source ~/.bashrc # Run this in every existing shell
```

#### Zsh

```sh
echo 'source <(env-select --shell zsh init)' >> ~/.zshrc
source ~/.zshrc # Run this in every existing shell
```

#### Fish

```sh
echo 'env-select --shell fish init | source' >> ~/.config/fish/config.fish
source ~/.config/fish/config.fish # Run this in every existing shell
```

**Restart your shell (or `source <file>`) after running the above command.**
