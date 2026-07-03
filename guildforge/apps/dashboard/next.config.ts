import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // The dashboard shells out to the `guildforge` CLI binary.
  // See ADR-0008 for the binding model.
  serverExternalPackages: [],
  experimental: {
    // Enable server actions for form handling.
    serverActions: {
      bodySizeLimit: "2mb",
    },
  },
};

export default nextConfig;
