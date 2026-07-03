/**
 * POST /api/plan — compute execution plan for a YAML config.
 *
 * Body: { yaml: string }
 * Response: { plan: object, exitCode: number }
 */

import { NextRequest, NextResponse } from "next/server";
import { planConfig } from "@/lib/cli";
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

  const { plan, raw } = await planConfig(yaml);
  return NextResponse.json({
    plan,
    exitCode: raw.exitCode,
    stdout: raw.stdout,
    stderr: raw.stderr,
  });
}
