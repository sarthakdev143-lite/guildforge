#!/usr/bin/env python3
"""Generate shell completions for guildforge.

Usage:
    python3 scripts/generate-completions.py [--output-dir DIR]

Output:
    DIR/{bash,zsh,fish,powershell}/_guildforge
"""

import subprocess
import sys
import os
from pathlib import Path

def main():
    output_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("assets/completions")

    bin_path = os.environ.get("GUILDFORGE_BIN", "target/release/guildforge")
    if not Path(bin_path).exists():
        bin_path = "target/debug/guildforge"
    if not Path(bin_path).exists():
        print("Error: could not find guildforge binary", file=sys.stderr)
        sys.exit(1)

    for shell in ["bash", "zsh", "fish", "powershell", "elvish"]:
        print(f"Generating {shell} completions...")
        shell_dir = output_dir / shell
        shell_dir.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [bin_path, "completions", shell],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"  Error: {result.stderr}", file=sys.stderr)
            sys.exit(1)
        ext = ".ps1" if shell == "powershell" else ""
        out_file = shell_dir / f"_guildforge{ext}"
        out_file.write_text(result.stdout)
        print(f"  -> {out_file}")

    print(f"\nCompletions generated in {output_dir}")

if __name__ == "__main__":
    main()
