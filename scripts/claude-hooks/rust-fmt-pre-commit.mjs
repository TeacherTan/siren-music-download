import {
  diffSnapshots,
  emitPreToolBlock,
  formatCommandOutput,
  formatFileList,
  getRepoRoot,
  getStagedFiles,
  getStagedRustFiles,
  getTrackedChangedFiles,
  getTrackedChangedRustFiles,
  isSuccessful,
  runCommand,
  snapshotFileHashes,
} from './rust-fmt-lib.mjs';

const repoRoot = getRepoRoot();

if (!repoRoot) {
  process.exit(0);
}

const PRETTIER_PATTERNS = [
  '*.js',
  '*.mjs',
  '*.cjs',
  '*.ts',
  '*.svelte',
  '*.md',
];

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
const prettierFiles = includesAllTrackedChanges
  ? [
      ...new Set([
        ...getStagedFiles(repoRoot, PRETTIER_PATTERNS),
        ...getTrackedChangedFiles(repoRoot, PRETTIER_PATTERNS),
      ]),
    ]
  : getStagedFiles(repoRoot, PRETTIER_PATTERNS);

if (rustFiles.length === 0 && prettierFiles.length === 0) {
  process.exit(0);
}

const targetRustFiles = [...new Set(rustFiles)].sort((left, right) =>
  left.localeCompare(right)
);
const targetPrettierFiles = [...new Set(prettierFiles)].sort((left, right) =>
  left.localeCompare(right)
);
const targetFiles = [
  ...new Set([...targetRustFiles, ...targetPrettierFiles]),
].sort((left, right) => left.localeCompare(right));
const beforeHashes = snapshotFileHashes(repoRoot, targetFiles);

if (targetPrettierFiles.length > 0) {
  const prettierWriteResult = runCommand(
    'bunx',
    ['prettier', '--write', ...targetPrettierFiles],
    repoRoot,
    process.env
  );

  if (!isSuccessful(prettierWriteResult)) {
    const commandOutput = formatCommandOutput(prettierWriteResult);
    const detailSuffix = commandOutput.length > 0 ? `\n\n${commandOutput}` : '';

    emitPreToolBlock({
      message: `Blocked git commit: \`prettier --write\` failed. Fix the formatter error, then retry the commit.${detailSuffix}`,
      stopReason: 'Blocked git commit because \`prettier --write\` failed.',
      reason: 'prettier --write failed',
      permissionDecisionReason: 'Run prettier successfully before committing.',
    });
    process.exit(0);
  }
}

if (targetRustFiles.length > 0) {
  const rustfmtResult = runCommand(
    'rustfmt',
    ['--edition', '2021', ...targetRustFiles],
    repoRoot,
    process.env
  );

  if (!isSuccessful(rustfmtResult)) {
    const commandOutput = formatCommandOutput(rustfmtResult);
    const detailSuffix = commandOutput.length > 0 ? `\n\n${commandOutput}` : '';

    emitPreToolBlock({
      message: `Blocked git commit: \`rustfmt --edition 2021 ...\` failed. Fix the formatter error, then retry the commit.${detailSuffix}`,
      stopReason: 'Blocked git commit because \`rustfmt\` failed.',
      reason: 'rustfmt failed',
      permissionDecisionReason: 'Run rustfmt successfully before committing.',
    });
    process.exit(0);
  }
}

const afterHashes = snapshotFileHashes(repoRoot, targetFiles);
const changedFiles = diffSnapshots(beforeHashes, afterHashes);

if (changedFiles.length > 0) {
  emitPreToolBlock({
    message: `Blocked git commit: staged files were reformatted by Prettier and/or rustfmt. Review the diff, restage the changed files, then rerun the commit.\n\nFiles changed by formatter:\n${formatFileList(changedFiles)}`,
    stopReason: 'Blocked git commit because formatters changed staged files.',
    reason: 'formatters updated staged files',
    permissionDecisionReason:
      'Formatters changed staged files; review and restage before committing.',
  });
}
