# Shell Completions

```bash
intent completions <shell>
```

Generates tab-completion scripts for your shell. Completions cover all subcommands, flags, and options.

## Supported shells

| Shell | Value |
|-------|-------|
| Bash | `bash` |
| Zsh | `zsh` |
| Fish | `fish` |
| PowerShell | `powershell` |
| Elvish | `elvish` |

## Setup

The command writes the completion script to stdout. Redirect it to the appropriate location for your shell.

### Bash

```bash
intent completions bash | sudo tee /usr/share/bash-completion/completions/intent > /dev/null
source /usr/share/bash-completion/completions/intent
```

Or for the current user only:

```bash
mkdir -p ~/.local/share/bash-completion/completions
intent completions bash > ~/.local/share/bash-completion/completions/intent
```

### Zsh

```bash
intent completions zsh > ~/.zfunc/_intent
```

Make sure `~/.zfunc` is in your `fpath` (add `fpath=(~/.zfunc $fpath)` to `~/.zshrc` before `compinit`).

### Fish

```bash
intent completions fish > ~/.config/fish/completions/intent.fish
```

### PowerShell

```powershell
intent completions powershell >> $PROFILE
```

## What's covered

Completions include:
- All subcommands (`check`, `render`, `compile`, `verify`, `serve`, etc.)
- Flags and options for each subcommand (`--write`, `--check`, `--addr`, etc.)
- Global options (`--output`, `--help`, `--version`)
