/**
 * Main dashboard page — YAML editor, plan viewer, apply.
 *
 * The page has three panels:
 * 1. YAML editor (left)
 * 2. Plan / validation output (right)
 * 3. Apply log (bottom, collapsible)
 */

"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { useRouter } from "next/navigation";

const DEFAULT_YAML = `server:
  name: My Guild

roles:
  - name: Admin
    color: red
    permissions: [administrator]

channels:
  - name: general
    type: text
    topic: General chat
`;

interface PlanOperation {
  op: string;
  [key: string]: unknown;
}

interface PlanSummary {
  create: number;
  update: number;
  delete: number;
  noop: number;
}

export default function DashboardPage() {
  const router = useRouter();
  const [yaml, setYaml] = useState(DEFAULT_YAML);
  const [validation, setValidation] = useState<{
    valid: boolean;
    stderr: string;
  } | null>(null);
  const [plan, setPlan] = useState<{
    plan: { operations?: PlanOperation[] } | null;
    exitCode: number;
    stderr: string;
  } | null>(null);
  const [applyLogs, setApplyLogs] = useState<string[]>([]);
  const [applying, setApplying] = useState(false);
  const [applyExitCode, setApplyExitCode] = useState<number | null>(null);
  const logRef = useRef<HTMLDivElement>(null);

  // Auto-scroll log panel.
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [applyLogs]);

  const handleValidate = useCallback(async () => {
    setValidation(null);
    const res = await fetch("/api/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ yaml }),
    });
    if (res.status === 401) {
      router.push("/login");
      return;
    }
    const data = await res.json();
    setValidation(data);
  }, [yaml, router]);

  const handlePlan = useCallback(async () => {
    setPlan(null);
    const res = await fetch("/api/plan", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ yaml }),
    });
    if (res.status === 401) {
      router.push("/login");
      return;
    }
    const data = await res.json();
    setPlan(data);
  }, [yaml, router]);

  const handleApply = useCallback(async () => {
    setApplying(true);
    setApplyLogs([]);
    setApplyExitCode(null);

    const res = await fetch("/api/apply", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ yaml }),
    });

    if (res.status === 401) {
      router.push("/login");
      return;
    }

    const reader = res.body?.getReader();
    const decoder = new TextDecoder();

    if (reader) {
      let buffer = "";
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        // Parse SSE events.
        const events = buffer.split("\n\n");
        buffer = events.pop() || "";

        for (const event of events) {
          const lines = event.split("\n");
          let eventType = "";
          let data = "";
          for (const line of lines) {
            if (line.startsWith("event: ")) eventType = line.slice(7);
            if (line.startsWith("data: ")) data = line.slice(6);
          }
          if (data) {
            try {
              const parsed = JSON.parse(data);
              const prefix =
                eventType === "stdout"
                  ? ""
                  : eventType === "stderr"
                    ? "[stderr] "
                    : eventType === "exit"
                      ? "[exit] "
                      : eventType === "error"
                        ? "[error] "
                        : `[${eventType}] `;
              setApplyLogs((prev) => [...prev, prefix + parsed]);
              if (eventType === "exit") {
                const match = /exit code: (\d+)/.exec(parsed);
                if (match) {
                  setApplyExitCode(parseInt(match[1], 10));
                }
              }
            } catch {
              // ignore
            }
          }
        }
      }
    }

    setApplying(false);
  }, [yaml, router]);

  const handleLogout = useCallback(async () => {
    await fetch("/api/logout", { method: "POST" });
    router.push("/login");
  }, [router]);

  // Compute plan summary.
  const summary: PlanSummary | null = plan?.plan?.operations
    ? plan.plan.operations.reduce(
        (acc: PlanSummary, op: PlanOperation) => {
          switch (op.op) {
            case "create":
              acc.create++;
              break;
            case "update":
              acc.update++;
              break;
            case "delete":
              acc.delete++;
              break;
            case "noop":
              acc.noop++;
              break;
          }
          return acc;
        },
        { create: 0, update: 0, delete: 0, noop: 0 }
      )
    : null;

  return (
    <div className="flex h-screen flex-col">
      {/* Header */}
      <header className="flex items-center justify-between border-b border-border px-6 py-3">
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-bold">GuildForge</h1>
          <span className="text-sm text-muted-foreground">
            Infrastructure as Code for Discord
          </span>
        </div>
        <div className="flex items-center gap-3">
          <a
            href="/history"
            className="text-sm text-muted-foreground hover:text-foreground"
          >
            History
          </a>
          <button
            onClick={handleLogout}
            className="text-sm text-muted-foreground hover:text-foreground"
          >
            Logout
          </button>
        </div>
      </header>

      {/* Main content — split layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left: YAML editor */}
        <div className="flex w-1/2 flex-col border-r border-border">
          <div className="flex items-center justify-between border-b border-border px-4 py-2">
            <span className="text-sm font-medium">config.yaml</span>
            <div className="flex gap-2">
              <button
                onClick={handleValidate}
                className="rounded-md bg-secondary px-3 py-1 text-xs font-medium text-secondary-foreground hover:bg-secondary/80"
              >
                Validate
              </button>
              <button
                onClick={handlePlan}
                className="rounded-md bg-secondary px-3 py-1 text-xs font-medium text-secondary-foreground hover:bg-secondary/80"
              >
                Plan
              </button>
              <button
                onClick={handleApply}
                disabled={applying}
                className="rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {applying ? "Applying..." : "Apply"}
              </button>
            </div>
          </div>
          <textarea
            value={yaml}
            onChange={(e) => setYaml(e.target.value)}
            className="flex-1 resize-none bg-background p-4 font-mono text-sm outline-none"
            spellCheck={false}
          />
        </div>

        {/* Right: Plan / validation output */}
        <div className="flex w-1/2 flex-col overflow-auto">
          {/* Validation result */}
          {validation && (
            <div className="border-b border-border p-4">
              <h3 className="mb-2 text-sm font-medium">Validation</h3>
              {validation.valid ? (
                <p className="text-sm text-green-500">✓ Config is valid</p>
              ) : (
                <pre className="whitespace-pre-wrap text-sm text-destructive">
                  {validation.stderr}
                </pre>
              )}
            </div>
          )}

          {/* Plan result */}
          {plan && (
            <div className="p-4">
              <div className="mb-3 flex items-center justify-between">
                <h3 className="text-sm font-medium">Execution Plan</h3>
                {summary && (
                  <span className="text-xs text-muted-foreground">
                    +{summary.create} ~{summary.update} -{summary.delete} ={summary.noop}
                  </span>
                )}
              </div>
              {plan.plan?.operations && plan.plan.operations.length > 0 ? (
                <div className="space-y-1">
                  {plan.plan.operations.map((op, i) => {
                    const symbol =
                      op.op === "create"
                        ? "+"
                        : op.op === "update"
                          ? "~"
                          : op.op === "delete"
                            ? "-"
                            : "=";
                    const color =
                      op.op === "create"
                        ? "text-green-500"
                        : op.op === "update"
                          ? "text-yellow-500"
                          : op.op === "delete"
                            ? "text-red-500"
                            : "text-muted-foreground";
                    // Extract address from the operation.
                    const addr =
                      (op.desired as { addr?: string })?.addr ||
                      (op.current as { addr?: string })?.addr ||
                      "unknown";
                    return (
                      <div key={i} className="flex items-center gap-2 font-mono text-sm">
                        <span className={`${color} font-bold`}>{symbol}</span>
                        <span className="text-muted-foreground">{op.op}</span>
                        <span>{addr}</span>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">
                  {plan.exitCode === 0
                    ? "No changes."
                    : `Exit code: ${plan.exitCode}`}
                </p>
              )}
              {plan.stderr && (
                <pre className="mt-2 whitespace-pre-wrap text-xs text-destructive">
                  {plan.stderr}
                </pre>
              )}
            </div>
          )}

          {/* Apply logs */}
          {(applyLogs.length > 0 || applying) && (
            <div className="border-t border-border p-4">
              <h3 className="mb-2 text-sm font-medium">
                Apply Log {applyExitCode !== null && `(exit ${applyExitCode})`}
              </h3>
              <div
                ref={logRef}
                className="max-h-64 overflow-auto rounded-md bg-black/20 p-3 font-mono text-xs"
              >
                {applyLogs.map((line, i) => (
                  <div
                    key={i}
                    className={
                      line.startsWith("[stderr]") || line.startsWith("[error]")
                        ? "text-red-400"
                        : line.startsWith("[exit]")
                          ? "text-yellow-400"
                          : "text-green-400"
                    }
                  >
                    {line}
                  </div>
                ))}
                {applying && (
                  <div className="animate-pulse text-muted-foreground">
                    ▋
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
