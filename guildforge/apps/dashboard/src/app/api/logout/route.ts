/**
 * POST /api/logout — clear session and delete stored token.
 */

import { NextResponse } from "next/server";
import { clearSession } from "@/lib/session";
import { deleteToken } from "@/lib/cli";

export async function POST() {
  await clearSession();
  await deleteToken();
  return NextResponse.json({ ok: true });
}
