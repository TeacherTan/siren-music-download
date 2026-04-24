import {
  diffSnapshots,
  emitPreToolBlock,
  formatCommandOutput,
  formatFileList,
  getRepoRoot,
  getStagedRustFiles,
  getTrackedChangedRustFiles,
  isSuccessful,
  runCommand,
  snapshotFileHashes,
} from './rust-fmt-lib.mjs';

const repoRoot = getRepoRoot();

if (!repoRoot) {
  process.exit(0);
}

const hookCommand = process.env.HOOK_COMMAND ?? '';
const includesAllTrackedChanges = /(?:^|\s)(?:-a|--all)(?=\s|$)/u.test(
  hookCommand
);
const rustFiles = includesAllTrackedChanges
  ? [
      ...new Set([
        ...getStagedRustFiles(repoRoot),
        ...getTrackedChangedRustFiles(repoRoot),
      ]),
    ]
  : getStagedRustFiles(repoRoot);

if (rustFiles.length === 0) {
  process.exit(0);
}

const targetRustFiles = [...new Set(rustFiles)].sort((left, right) =>
  left.localeCompare(right)
);
const beforeHashes = snapshotFileHashes(repoRoot, targetRustFiles);
const formatResult = runCommand(
  'rustfmt',
  ['--edition', '2021', ...targetRustFiles],
  repoRoot,
  process.env
);

if (!isSuccessful(formatResult)) {
  const commandOutput = formatCommandOutput(formatResult);
  const detailSuffix = commandOutput.length > 0 ? `\n\n${commandOutput}` : '';

  emitPreToolBlock({
    message: `Blocked git commit: \`rustfmt --edition 2021 ...\` failed. Fix the formatter error, then retry the commit.${detailSuffix}`,
    stopReason: 'Blocked git commit because \`rustfmt\` failed.',
    reason: 'rustfmt failed',
    permissionDecisionReason: 'Run rustfmt successfully before committing.',
  });
  process.exit(0);
}

const afterHashes = snapshotFileHashes(repoRoot, targetRustFiles);
const changedFiles = diffSnapshots(beforeHashes, afterHashes);

if (changedFiles.length > 0) {
  emitPreToolBlock({
    message: `Blocked git commit: Rust files were reformatted by \`rustfmt\`. Review the diff, restage the changed files, then rerun the commit.\n\nFiles changed by formatter:\n${formatFileList(changedFiles)}`,
    stopReason: 'Blocked git commit because rustfmt changed Rust files.',
    reason: 'rustfmt updated Rust files',
    permissionDecisionReason:
      'rustfmt changed Rust files; review and restage before committing.',
  });
}
