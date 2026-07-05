# How to Publish GuildForge to GitHub

You have a **git bundle** (`guildforge-v1.0.0.git-bundle`, 261 KB) that
contains the entire repository with full history and the `v1.0.0` tag.

## Step 1: Create a GitHub repository

1. Go to https://github.com/new
2. Name it `guildforge` (or whatever you want)
3. Set it to **Public** (required for free GitHub Actions)
4. **Do NOT** check "Add a README" or "Add .gitignore" — leave it empty
5. Click **Create repository**

## Step 2: Clone the bundle to your machine

You need `git` installed (it's tiny — no Rust needed). Download
`guildforge-v1.0.0.git-bundle` from the workspace, then:

```bash
# Clone from the bundle (creates a new folder called guildforge)
git clone guildforge-v1.0.0.git-bundle guildforge

cd guildforge
```

## Step 3: Push to GitHub

```bash
# Add your GitHub repo as the remote
git remote add origin https://github.com/YOUR-USERNAME/guildforge.git

# Push the main branch
git push -u origin main

# Push the v1.0.0 tag (this triggers the release CI!)
git push origin v1.0.0
```

## Step 4: Watch the CI build binaries

Go to your repo's **Actions** tab:
```
https://github.com/YOUR-USERNAME/guildforge/actions
```

You'll see two workflows running:

1. **CI** — runs tests, clippy, fmt, deny, audit on every push
2. **Release** — triggered by the `v1.0.0` tag, builds binaries for:
   - Linux x86_64
   - Linux ARM64
   - macOS Intel
   - macOS Apple Silicon
   - Windows x86_64

The release workflow takes ~10 minutes. When it finishes, it creates a
**GitHub Release** at:
```
https://github.com/YOUR-USERNAME/guildforge/releases/tag/v1.0.0
```

## Step 5: Download your binary

Go to the Releases page and download the archive for your platform:

| Your OS | Download this file |
|---|---|
| Linux (Intel/AMD) | `guildforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM/Raspberry Pi) | `guildforge-v1.0.0-aarch64-unknown-linux-gnu.tar.gz` |
| Mac (Intel) | `guildforge-v1.0.0-x86_64-apple-darwin.tar.gz` |
| Mac (M1/M2/M3) | `guildforge-v1.0.0-aarch64-apple-darwin.tar.gz` |
| Windows | `guildforge-v1.0.0-x86_64-pc-windows-msvc.zip` |

Extract and run:
```bash
# Linux/macOS
tar xzf guildforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz
cd guildforge-v1.0.0-x86_64-unknown-linux-gnu
./bin/guildforge --help
./bin/guildforge version

# Windows
# Right-click the .zip → Extract All
# Open Command Prompt:
guildforge.exe --help
```

## Step 6: Start using it

```bash
# Store your Discord bot token
echo "YOUR_BOT_TOKEN" | ./bin/guildforge login

# Scaffold a config
./bin/guildforge init --template company

# Edit guildforge.yaml, then:
./bin/guildforge validate guildforge.yaml
./bin/guildforge plan guildforge.yaml
./bin/guildforge apply --auto-approve guildforge.yaml
```

## Troubleshooting

### CI workflows don't appear
Make sure the repo is **Public**. GitHub Actions requires the repo
to be public for free accounts, or you need a GitHub Pro/Team account
for private repos.

### Release workflow didn't trigger
The release workflow only runs on tags. Make sure you pushed the tag:
```bash
git push origin v1.0.0
```

### Build failed for one platform
Check the Actions log. The most common issue is a dependency that
doesn't compile on one platform. If aarch64-linux fails, that's the
cross-compilation target — it uses `cross` and may need Docker on
the CI runner (which GitHub provides).

### No Rust needed on your machine!
The entire build happens on GitHub's servers. You just download the
finished binary.
