/**
 * POST /api/login — authenticate with a passphrase.
 *
 * Body: { passphrase: string, token?: string }
 *
 * If a bot token is provided, it's stored encrypted on the server.
 * The passphrase sets the session cookie.
 */

import { NextRequest, NextResponse } from "next/server";
import { verifyPassphrase, setSession } from "@/lib/session";
import { storeToken } from "@/lib/cli";

export async function POST(req: NextRequest) {
  const body = await req.json();
  const passphrase = body?.passphrase;
  const token = body?.token;

  if (!verifyPassphrase(passphrase || "")) {
    return NextResponse.json(
      { error: "invalid passphrase" },
      { status: 403 }
    );
  }

  await setSession();

  // If a bot token was provided, store it encrypted.
  if (typeof token === "string" && token.length > 0) {
    await storeToken(token);
  }

  return NextResponse.json({ ok: true });
}
