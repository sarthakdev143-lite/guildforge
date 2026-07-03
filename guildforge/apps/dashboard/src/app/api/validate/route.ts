/**
 * POST /api/validate — validate a YAML config.
 *
 * Body: { yaml: string }
 * Response: { valid: boolean, diagnostics: string }
 */

import { NextRequest, NextResponse } from "next/server";
import { validateConfig } from "@/lib/cli";
import { isAuthenticated } from "@/lib/session";

export async function POST(req: NextRequest) {
  const authed = await isAuthenticated();
  if (!authed) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const body = await req.json();
  const yaml = body?.yaml;
  if (typeof yaml !== "string") {
    return NextResponse.json(
      { error: "missing 'yaml' field" },
      { status: 400 }
    );
  }

  const result = await validateConfig(yaml);
  return NextResponse.json({
    valid: result.exitCode === 0,
    exitCode: result.exitCode,
    stdout: result.stdout,
    stderr: result.stderr,
  });
}
