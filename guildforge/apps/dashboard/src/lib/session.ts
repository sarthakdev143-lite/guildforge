/**
 * Session management — simple passphrase-based auth.
 *
 * In v1, the dashboard is single-user. The passphrase is set via
 * `GUILDFORGE_DASHBOARD_PASS` env var. If unset, auth is disabled
 * (for local dev only).
 */

import { cookies } from "next/headers";
import { createHmac, timingSafeEqual } from "crypto";

const SESSION_COOKIE = "guildforge-session";
const SESSION_SECRET = process.env.GUILDFORGE_DASHBOARD_PASS || "";

/**
 * Verify a passphrase against the configured secret.
 */
export function verifyPassphrase(passphrase: string): boolean {
  if (!SESSION_SECRET) {
    // No passphrase configured — auth disabled (dev mode).
    return true;
  }
  const hash = createHmac("sha256", SESSION_SECRET)
    .update(passphrase)
    .digest();
  const expected = createHmac("sha256", SESSION_SECRET)
    .update(SESSION_SECRET)
    .digest();
  try {
    return timingSafeEqual(hash, expected);
  } catch {
    return false;
  }
}

/**
 * Check if the current request is authenticated.
 */
export async function isAuthenticated(): Promise<boolean> {
  if (!SESSION_SECRET) {
    return true; // Auth disabled.
  }
  const store = await cookies();
  const session = store.get(SESSION_COOKIE);
  if (!session) {
    return false;
  }
  const hash = createHmac("sha256", SESSION_SECRET)
    .update(session.value)
    .digest();
  const expected = createHmac("sha256", SESSION_SECRET)
    .update("authenticated")
    .digest();
  try {
    return timingSafeEqual(hash, expected);
  } catch {
    return false;
  }
}

/**
 * Set the session cookie after successful login.
 */
export async function setSession(): Promise<void> {
  const store = await cookies();
  store.set(SESSION_COOKIE, "authenticated", {
    httpOnly: true,
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
    maxAge: 60 * 60 * 24 * 7, // 7 days
  });
}

/**
 * Clear the session cookie (logout).
 */
export async function clearSession(): Promise<void> {
  const store = await cookies();
  store.delete(SESSION_COOKIE);
}
