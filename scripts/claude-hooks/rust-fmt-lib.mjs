import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export function getRepoRoot() {
  const result = runCommand("git", ["rev-parse", "--show-toplevel"]);

  if (!isSuccessful(result)) {
    return null;
  }

  const repoRoot = result.stdout.trim();
  return repoRoot.length > 0 ? repoRoot : null;
}

export function getChangedRustFiles(repoRoot) {
  const files = new Set([
    ...getTrackedChangedRustFiles(repoRoot),
    ...getStagedRustFiles(repoRoot),
    ...readGitPaths(repoRoot, ["ls-files", "--others", "--exclude-standard", "--", "*.rs"]),
  ]);

  return sortPaths(files);
}

export function getStagedRustFiles(repoRoot) {
  return sortPaths(readGitPaths(repoRoot, ["diff", "--cached", "--name-only", "--diff-filter=ACMR", "--", "*.rs"]));
}

export function getTrackedChangedRustFiles(repoRoot) {
  return sortPaths(readGitPaths(repoRoot, ["diff", "--name-only", "--diff-filter=ACMR", "--", "*.rs"]));
}

export function getWorkspaceRustFiles(repoRoot) {
  const files = new Set([
    ...readGitPaths(repoRoot, ["ls-files", "--", "*.rs"]),
    ...readGitPaths(repoRoot, ["ls-files", "--others", "--exclude-standard", "--", "*.rs"]),
  ]);

  return sortPaths(files);
}

export function snapshotFileHashes(repoRoot, files) {
  return new Map(files.map((file) => [file, hashFile(repoRoot, file)]));
}

export function diffSnapshots(before, after) {
  return sortPaths(
    [...before.entries()]
      .filter(([file, beforeHash]) => beforeHash !== after.get(file))
      .map(([file]) => file),
  );
}

export function runCommand(command, args, cwd, env) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    env,
  });

  return {
    errorMessage: result.error?.message ?? null,
    status: result.status,
    stderr: result.stderr ?? "",
    stdout: result.stdout ?? "",
  };
}

export function isSuccessful(result) {
  return result.errorMessage === null && result.status === 0;
}

export function formatCommandOutput(result) {
  return [result.stdout.trim(), result.stderr.trim()].filter(Boolean).join("\n\n");
}

export function formatFileList(files) {
  return files.map((file) => `- ${file}`).join("\n");
}

export function emitPreToolBlock(payload) {
  emitJson({
    systemMessage: payload.message,
    continue: false,
    stopReason: payload.stopReason,
    decision: "block",
    reason: payload.reason,
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      additionalContext: payload.message,
      permissionDecision: "deny",
      permissionDecisionReason: payload.permissionDecisionReason,
    },
  });
}

export function emitStopMessage(message) {
  emitJson({
    systemMessage: message,
    hookSpecificOutput: {
      hookEventName: "Stop",
      additionalContext: message,
    },
  });
}

function emitJson(payload) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function readGitPaths(repoRoot, args) {
  const result = runCommand("git", args, repoRoot);

  if (!isSuccessful(result)) {
    return [];
  }

  return result.stdout
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

function hashFile(repoRoot, relativePath) {
  const absolutePath = join(repoRoot, relativePath);

  if (!existsSync(absolutePath)) {
    return "__missing__";
  }

  return createHash("sha256").update(readFileSync(absolutePath)).digest("hex");
}

function sortPaths(paths) {
  return [...paths].sort((left, right) => left.localeCompare(right));
}
