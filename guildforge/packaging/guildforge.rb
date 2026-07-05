# Homebrew formula for GuildForge.
#
# To install via Homebrew:
#   brew tap your-org/guildforge
#   brew install guildforge
#
# This formula downloads a pre-built binary from GitHub Releases.

class Guildforge < Formula
  desc "Infrastructure as Code for Discord Workspaces"
  homepage "https://github.com/your-org/guildforge"
  version "1.0.0"
  license "MIT OR Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/your-org/guildforge/releases/download/v#{version}/guildforge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_REAL_SHA256"
    end
    on_intel do
      url "https://github.com/your-org/guildforge/releases/download/v#{version}/guildforge-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_REAL_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/your-org/guildforge/releases/download/v#{version}/guildforge-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_REAL_SHA256"
    end
    on_intel do
      url "https://github.com/your-org/guildforge/releases/download/v#{version}/guildforge-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_REAL_SHA256"
    end
  end

  def install
    bin.install "bin/guildforge"

    # Install shell completions.
    bash_completion.install "completions/bash/_guildforge"
    zsh_completion.install "completions/zsh/_guildforge"
    fish_completion.install "completions/fish/_guildforge"
  end

  test do
    assert_match "guildforge #{version}", shell_output("#{bin}/guildforge version")
  end
end
