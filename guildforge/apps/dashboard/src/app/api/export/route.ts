/**
 * GET /api/export — export current state as YAML.
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
    stderr: result.stderr,
  });
}
