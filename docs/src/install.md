# Install

See [the artifacts page](/artifacts) to download and install `es` using your preferred method.

## Install Shell Function

While not strictly required, it's highly recommended to install the `es` shell function. This wraps the `es` binary command, allowing it to automatically modify your current shell environment with the `es set` subcommand. Otherwise, you'll have to manually pipe the output of `es set` to `source`.

> If you only plan to use the `es run` command, this is **not relevant**.

This is necessary because a child process is not allowed to modify its parent's environment. That means the `es` process cannot modify the environment of the invoking shell. The wrapping shell function takes the output of `es` and runs it in that shell session to update the environment.

Here's how you install it:

### Bash

```sh
echo 'eval "$(es --shell bash init)"' >> ~/.bashrc
```

### Zsh

```sh
echo "source <(es --shell zsh init)" >> ~/.zshrc
```

### Fish

```sh
echo "es --shell fish init | source" >> ~/.config/fish/config.fish
```

**Restart your shell afterward to apply changes.**
