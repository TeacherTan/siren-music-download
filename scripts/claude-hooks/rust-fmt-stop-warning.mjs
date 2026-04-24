import {
  emitStopMessage,
  formatFileList,
  getChangedFiles,
  getChangedRustFiles,
  getRepoRoot,
  isSuccessful,
  runCommand,
} from './rust-fmt-lib.mjs';

const PRETTIER_PATTERNS = [
  '*.js',
  '*.mjs',
  '*.cjs',
  '*.ts',
  '*.svelte',
  '*.md',
];

const repoRoot = getRepoRoot();

if (!repoRoot) {
  process.exit(0);
}

const rustFiles = getChangedRustFiles(repoRoot);
const prettierFiles = getChangedFiles(repoRoot, PRETTIER_PATTERNS);
const reminderSections = [];

if (prettierFiles.length > 0) {
  const prettierCheckResult = runCommand(
    'bun',
    ['run', 'format:check'],
    repoRoot,
    process.env
  );

  if (!isSuccessful(prettierCheckResult)) {
    reminderSections.push(
      `Prettier-compatible formatting check failed for a workspace containing local JS/TS/Svelte/Markdown changes:\n${formatFileList(prettierFiles)}`
    );
  }
}

if (rustFiles.length > 0) {
  const rustfmtCheckResult = runCommand(
    'cargo',
    ['fmt', '--all', '--check'],
    repoRoot,
    process.env
  );

  if (!isSuccessful(rustfmtCheckResult)) {
    reminderSections.push(
      `Rust formatting check failed for a workspace containing local Rust changes:\n${formatFileList(rustFiles)}`
    );
  }
}

if (reminderSections.length === 0) {
  process.exit(0);
}

emitStopMessage(
  `Reminder: local changes do not satisfy the formatting checks mirrored from CI.\n\n${reminderSections.join('\n\n')}`
);
