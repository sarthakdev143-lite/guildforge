/**
 * GET /api/doctor — run drift detection.
 */

import { NextResponse } from "next/server";
import { runDoctor } from "@/lib/cli";
import { isAuthenticated } from "@/lib/session";

export async function GET() {
  const authed = await isAuthenticated();
  if (!authed) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const result = await runDoctor();
  return NextResponse.json({
    exitCode: result.exitCode,
    stdout: result.stdout,
    stderr: result.stderr,
  });
}
