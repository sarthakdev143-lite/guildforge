/**
 * CLI wrapper — shells out to the `guildforge` binary.
 *
 * Per ADR-0008, the dashboard never embeds the Rust engine in-process.
 * Every operation is a subprocess call to `guildforge`. The bot token
 * is passed via `GUILDFORGE_BOT_TOKEN` env var and never reaches the
 * browser.
 */

import { spawn, type ChildProcess } from "child_process";
import { readFile, writeFile, mkdir } from "fs/promises";
import { existsSync } from "fs";
import path from "path";
import os from "os";
import { randomBytes, createCipheriv, createDecipheriv, scryptSync } from "crypto";

// ===========================================================================
// Token storage — encrypted at rest, never sent to the browser.
// ===========================================================================

const TOKEN_DIR = path.join(os.homedir(), ".config", "guildforge");
const TOKEN_FILE = path.join(TOKEN_DIR, "token.enc");
// The encryption key is derived from a machine-specific identifier.
// In production (Phase 6+), this becomes the OS keychain via `keyring`.
const KEY_SALT = "guildforge-dashboard-v1";

function getEncryptionKey(): Buffer {
  // Derive a key from the username + hostname. This is NOT secure
  // against local attackers but prevents casual file-read attacks.
  // Phase 6 replaces this with the OS keychain.
  const machineId = `${os.userInfo().username}@${os.hostname()}`;
  return scryptSync(machineId, KEY_SALT, 32);
}

/**
 * Store the bot token encrypted on disk.
 * The token NEVER appears in browser-visible code or state.
 */
export async function storeToken(token: string): Promise<void> {
  await mkdir(TOKEN_DIR, { recursive: true });
  const key = getEncryptionKey();
  const iv = randomBytes(16);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  const encrypted = Buffer.concat([
    cipher.update(token, "utf8"),
    cipher.final(),
  ]);
  const authTag = cipher.getAuthTag();
  // File format: [iv (16 bytes)] [authTag (16 bytes)] [encrypted data]
  await writeFile(TOKEN_FILE, Buffer.concat([iv, authTag, encrypted]), {
    mode: 0o600,
  });
}

/**
 * Read the stored bot token. Returns null if no token is stored.
 * This function is ONLY callable from server-side code.
 */
export async function readToken(): Promise<string | null> {
  if (!existsSync(TOKEN_FILE)) {
    return null;
  }
  try {
    const data = await readFile(TOKEN_FILE);
    const iv = data.subarray(0, 16);
    const authTag = data.subarray(16, 32);
    const encrypted = data.subarray(32);
    const key = getEncryptionKey();
    const decipher = createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAuthTag(authTag);
    const decrypted = Buffer.concat([
      decipher.update(encrypted),
      decipher.final(),
    ]);
    return decrypted.toString("utf8");
  } catch {
    return null;
  }
}

/**
 * Delete the stored token.
 */
export async function deleteToken(): Promise<void> {
  if (existsSync(TOKEN_FILE)) {
    await writeFile(TOKEN_FILE, "", { mode: 0o600 });
  }
}

/**
 * Check if a token is stored.
 */
export function hasToken(): boolean {
  return existsSync(TOKEN_FILE);
}

// ===========================================================================
// CLI execution
// ===========================================================================

/** Path to the guildforge binary. */
const GUILDFORGE_BIN = process.env.GUILDFORGE_BIN || "guildforge";

/** Default state file path (per-session temp file). */
const STATE_FILE = path.join(
  os.tmpdir(),
  `guildforge-${process.pid}.db`
);

export interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

/**
 * Run `guildforge` with the given arguments.
 *
 * The bot token (if stored) is passed via env var and NEVER appears in
 * the arguments (which would be visible via `ps`).
 */
export async function runGuildforge(
  args: string[],
  options?: { stdin?: string; timeout?: number }
): Promise<CliResult> {
  const token = await readToken();
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    GUILDFORGE_STATE_FILE: STATE_FILE,
    GUILDFORGE_NO_NETWORK: "",
  };
  if (token) {
    env.GUILDFORGE_BOT_TOKEN = token;
  }

  return new Promise((resolve, reject) => {
    const proc = spawn(GUILDFORGE_BIN, args, {
      env,
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    const timeout = options?.timeout ?? 30_000;
    const timer = setTimeout(() => {
      proc.kill("SIGTERM");
      reject(new Error(`guildforge timed out after ${timeout}ms`));
    }, timeout);

    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });
    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });
    proc.on("error", (err) => {
      clearTimeout(timer);
      reject(new Error(`could not spawn guildforge: ${err.message}`));
    });
    proc.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        exitCode: code ?? 1,
        stdout,
        stderr,
      });
    });

    if (options?.stdin) {
      proc.stdin.write(options.stdin);
      proc.stdin.end();
    } else {
      proc.stdin.end();
    }
  });
}

/**
 * Run `guildforge validate <file>` and return the result.
 */
export async function validateConfig(yamlContent: string): Promise<CliResult> {
  // Write YAML to a temp file, then validate.
  const tmpFile = path.join(os.tmpdir(), `gf-validate-${Date.now()}.yaml`);
  await writeFile(tmpFile, yamlContent);
  try {
    return await runGuildforge(["validate", tmpFile]);
  } finally {
    // Best-effort cleanup.
    try {
      await writeFile(tmpFile, "", { mode: 0o600 });
    } catch {}
  }
}

/**
 * Run `guildforge plan <file> --format json` and return the parsed plan.
 */
export async function planConfig(
  yamlContent: string
): Promise<{ plan: unknown; raw: CliResult }> {
  const tmpFile = path.join(os.tmpdir(), `gf-plan-${Date.now()}.yaml`);
  await writeFile(tmpFile, yamlContent);
  try {
    const result = await runGuildforge(["plan", tmpFile, "--format", "json"]);
    let plan: unknown = null;
    if (result.stdout.trim()) {
      try {
        plan = JSON.parse(result.stdout);
      } catch {
        // Non-JSON output (e.g. error message)
      }
    }
    return { plan, raw: result };
  } finally {
    try {
      await writeFile(tmpFile, "", { mode: 0o600 });
    } catch {}
  }
}

/**
 * Run `guildforge apply <file> --auto-approve` and stream output.
 * Returns a ChildProcess for WebSocket bridging.
 */
export function applyConfigStream(
  yamlContent: string
): ChildProcess {
  const tmpFile = path.join(os.tmpdir(), `gf-apply-${Date.now()}.yaml`);
  // Write synchronously for simplicity in streaming context.
  const { writeFileSync } = require("fs");
  writeFileSync(tmpFile, yamlContent);

  const token = readTokenSync();
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    GUILDFORGE_STATE_FILE: STATE_FILE,
  };
  if (token) {
    env.GUILDFORGE_BOT_TOKEN = token;
  }

  return spawn(
    GUILDFORGE_BIN,
    ["apply", tmpFile, "--auto-approve"],
    { env, stdio: ["pipe", "pipe", "pipe"] }
  );
}

/** Synchronous token read (for streaming contexts). */
function readTokenSync(): string | null {
  if (!existsSync(TOKEN_FILE)) {
    return null;
  }
  try {
    const { readFileSync } = require("fs");
    const data = readFileSync(TOKEN_FILE);
    const iv = data.subarray(0, 16);
    const authTag = data.subarray(16, 32);
    const encrypted = data.subarray(32);
    const key = getEncryptionKey();
    const decipher = createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAuthTag(authTag);
    const decrypted = Buffer.concat([
      decipher.update(encrypted),
      decipher.final(),
    ]);
    return decrypted.toString("utf8");
  } catch {
    return null;
  }
}

/**
 * Run `guildforge doctor` and return the result.
 */
export async function runDoctor(): Promise<CliResult> {
  return runGuildforge(["doctor"]);
}

/**
 * Run `guildforge version` and return the result.
 */
export async function runVersion(): Promise<CliResult> {
  return runGuildforge(["version"]);
}

/**
 * Run `guildforge export` and return the YAML.
 */
export async function runExport(): Promise<CliResult> {
  return runGuildforge(["export"]);
}
