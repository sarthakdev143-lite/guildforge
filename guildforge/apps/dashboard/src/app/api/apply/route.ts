/**
 * POST /api/apply — apply a YAML config.
 *
 * Body: { yaml: string }
 * Response: Server-Sent Events stream of apply output.
 *
 * This route spawns `guildforge apply --auto-approve` and streams
 * stdout/stderr lines as SSE events. The browser receives real-time
 * progress updates.
 */

import { NextRequest } from "next/server";
import { applyConfigStream } from "@/lib/cli";
import { isAuthenticated } from "@/lib/session";
import type { ChildProcess } from "child_process";

export async function POST(req: NextRequest) {
  const authed = await isAuthenticated();
  if (!authed) {
    return new Response("unauthorized", { status: 401 });
  }

  const body = await req.json();
  const yaml = body?.yaml;
  if (typeof yaml !== "string") {
    return new Response("missing 'yaml' field", { status: 400 });
  }

  const proc: ChildProcess = applyConfigStream(yaml);

  const stream = new ReadableStream({
    start(controller) {
      const encoder = new TextEncoder();
      const send = (event: string, data: string) => {
        controller.enqueue(
          encoder.encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
        );
      };

      send("start", "apply started");

      proc.stdout?.on("data", (data) => {
        const lines = data.toString().split("\n").filter(Boolean);
        for (const line of lines) {
          send("stdout", line);
        }
      });

      proc.stderr?.on("data", (data) => {
        const lines = data.toString().split("\n").filter(Boolean);
        for (const line of lines) {
          send("stderr", line);
        }
      });

      proc.on("close", (code) => {
        send("exit", `exit code: ${code}`);
        controller.close();
      });

      proc.on("error", (err) => {
        send("error", err.message);
        controller.close();
      });
    },
    cancel() {
      proc.kill("SIGTERM");
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
}
