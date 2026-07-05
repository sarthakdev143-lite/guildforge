FROM debian:bookworm-slim

# Copy the pre-built binary
COPY guildforge-linux-x86_64 /usr/local/bin/guildforge

# Make it executable
RUN chmod +x /usr/local/bin/guildforge

# Install ca-certificates for HTTPS (needed for Discord API)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a working directory
WORKDIR /workspace

# Default command
ENTRYPOINT ["guildforge"]
CMD ["--help"]
