/**
 * GET /api/version — get guildforge version info.
 */

import { NextResponse } from "next/server";
import { runVersion } from "@/lib/cli";

export async function GET() {
  const result = await runVersion();
  return NextResponse.json({
    version: result.stdout,
    exitCode: result.exitCode,
  });
}
