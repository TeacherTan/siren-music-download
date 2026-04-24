import {
  emitStopMessage,
  formatFileList,
  getChangedRustFiles,
  getRepoRoot,
  isSuccessful,
  runCommand,
} from './rust-fmt-lib.mjs';

const repoRoot = getRepoRoot();

if (!repoRoot) {
  process.exit(0);
}

const rustFiles = getChangedRustFiles(repoRoot);

if (rustFiles.length === 0) {
  process.exit(0);
}

const formatCheckResult = runCommand(
  'cargo',
  ['fmt', '--all', '--check'],
  repoRoot
);

if (isSuccessful(formatCheckResult)) {
  process.exit(0);
}

emitStopMessage(
  `Reminder: modified Rust files appear unformatted. Run \`cargo fmt --all\` before considering this work complete.\n\nRust files with local changes:\n${formatFileList(rustFiles)}`
);
