# Installation

## From source (recommended for development)

```bash
cargo install --path apps/cli
```

## From GitHub Releases

Download the appropriate archive for your platform from the
[Releases page](https://github.com/your-org/guildforge/releases):

| Platform | Archive |
|---|---|
| Linux x86_64 | `guildforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `guildforge-v1.0.0-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `guildforge-v1.0.0-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 (Apple Silicon) | `guildforge-v1.0.0-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `guildforge-v1.0.0-x86_64-pc-windows-msvc.zip` |

Extract and add to your PATH:

```bash
tar xzf guildforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv guildforge-v1.0.0-x86_64-unknown-linux-gnu/bin/guildforge /usr/local/bin/
```

## Via Homebrew (macOS)

```bash
brew tap your-org/guildforge
brew install guildforge
```

## Via Scoop (Windows)

```bash
scoop bucket add guildforge https://github.com/your-org/guildforge
scoop install guildforge
```

## Shell completions

After installation, enable shell completions:

### Bash

```bash
guildforge completions bash > /etc/bash_completion.d/guildforge
# or for user-level:
guildforge completions bash > ~/.local/share/bash-completion/completions/guildforge
```

### Zsh

```bash
guildforge completions zsh > "${fpath[1]}/_guildforge"
```

### Fish

```bash
guildforge completions fish > ~/.config/fish/completions/guildforge.fish
```

## Authentication

Store your Discord bot token:

```bash
guildforge login
# Enter your bot token when prompted
```

The token is stored at `~/.config/guildforge/token` with mode 0600.
With `--features keychain`, the OS keychain is used instead.

## Verify

```bash
guildforge version
guildforge --help
```
