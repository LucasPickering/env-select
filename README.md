# env-select

![license](https://img.shields.io/github/license/LucasPickering/env-select)
[![crates.io version](https://img.shields.io/crates/v/env-select.svg)](https://crates.io/crates/env-select)

Easily switch between predefined values for arbitrary environment variables.

## Usage

First, define `.env-select.toml`. This is where you'll specify possible options for each variable. Here's an example:

```toml
[variables]
TEST_VARIABLE = ["abc", "def"]
```

Now, you can easily switch between the defined values (or specify an adhoc value) with `es`:

```sh
> es TEST_VARIABLE | source
? TEST_VARIABLE= ›
❯ abc
  def
  Custom
✔ TEST_VARIABLE= · abc
> echo $TEST_VARIABLE
abc
```
