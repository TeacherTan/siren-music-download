import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const raw = await new Response(process.stdin).text();
const payload = raw ? JSON.parse(raw) : {};
const cwd = typeof payload?.cwd === "string" && payload.cwd.trim() ? payload.cwd.trim() : process.cwd();
const repo = spawnSync("git", ["rev-parse", "--show-toplevel"], {
  cwd,
  encoding: "utf8",
});

if (repo.error || repo.status !== 0) {
  process.exit(0);
}

const root = repo.stdout.trim();
const script = resolve(scriptDirectory, "rust-fmt-stop-warning.mjs");
const result = spawnSync(process.execPath, [script], {
  cwd: root,
  encoding: "utf8",
});

if (result.stdout) {
  process.stdout.write(result.stdout);
}

if (result.stderr) {
  process.stderr.write(result.stderr);
}

if (result.error) {
  process.stderr.write(`${result.error.message}\n`);
  process.exit(1);
}

process.exit(result.status ?? 1);
