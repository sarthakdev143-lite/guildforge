/**
 * History page — shows exported state YAML.
 *
 * In v1, this calls `guildforge export` and displays the result.
 * Phase 6+ will add a proper migrations_log viewer.
 */

"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";

export default function HistoryPage() {
  const router = useRouter();
  const [yaml, setYaml] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const fetchHistory = useCallback(async () => {
    setLoading(true);
    const res = await fetch("/api/history");
    if (res.status === 401) {
      router.push("/login");
      return;
    }
    const data = await res.json();
    setYaml(data.yaml || "");
    if (data.exitCode !== 0) {
      setError(data.stderr || "could not export state");
    }
    setLoading(false);
  }, [router]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border px-6 py-3">
        <div className="flex items-center gap-4">
          <a href="/" className="text-lg font-bold hover:opacity-80">
            ← GuildForge
          </a>
          <span className="text-sm text-muted-foreground">History</span>
        </div>
        <button
          onClick={() => router.push("/")}
          className="text-sm text-muted-foreground hover:text-foreground"
        >
          Back to Dashboard
        </button>
      </header>

      <div className="flex-1 overflow-auto p-6">
        <h2 className="mb-4 text-lg font-medium">Exported State</h2>
        {loading ? (
          <p className="text-sm text-muted-foreground">Loading...</p>
        ) : error ? (
          <div className="rounded-md border border-destructive/50 bg-destructive/10 p-4">
            <p className="text-sm text-destructive">{error}</p>
            <p className="mt-2 text-xs text-muted-foreground">
              Make sure the guildforge binary is on PATH and a bot token
              has been configured.
            </p>
          </div>
        ) : (
          <pre className="rounded-md bg-black/20 p-4 font-mono text-sm overflow-auto">
            {yaml || "(empty state)"}
          </pre>
        )}
      </div>
    </div>
  );
}
