/**
 * GET /api/history — migration history from state.
 *
 * Runs `guildforge export` and parses the state's migration log.
 * In v1, we return the raw export; the frontend formats it.
 */

import { NextResponse } from "next/server";
import { runExport } from "@/lib/cli";
import { isAuthenticated } from "@/lib/session";

export async function GET() {
  const authed = await isAuthenticated();
  if (!authed) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const result = await runExport();
  return NextResponse.json({
    yaml: result.stdout,
    exitCode: result.exitCode,
  });
}
