import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const raw = await new Response(process.stdin).text();
const payload = raw ? JSON.parse(raw) : {};
const command = payload?.tool_input?.command ?? '';
const cwd =
  typeof payload?.cwd === 'string' && payload.cwd.trim()
    ? payload.cwd.trim()
    : process.cwd();
const quoted = command.match(/(?:^|\s)git\s+-C\s+"([^"]+)"/u);
const bare = command.match(/(?:^|\s)git\s+-C\s+(\S+)/u);
const candidate = quoted?.[1] ?? bare?.[1] ?? '';
const fromCandidate = candidate
  ? spawnSync('git', ['-C', candidate, 'rev-parse', '--show-toplevel'], {
      encoding: 'utf8',
    })
  : null;
const fromCwd = spawnSync('git', ['rev-parse', '--show-toplevel'], {
  cwd,
  encoding: 'utf8',
});
const root =
  fromCandidate && !fromCandidate.error && fromCandidate.status === 0
    ? fromCandidate.stdout.trim()
    : !fromCwd.error && fromCwd.status === 0
      ? fromCwd.stdout.trim()
      : '';

if (!root) {
  process.exit(0);
}

const script = resolve(scriptDirectory, 'rust-fmt-pre-commit.mjs');
const result = spawnSync(process.execPath, [script], {
  cwd: root,
  encoding: 'utf8',
  env: {
    ...process.env,
    HOOK_COMMAND: command,
  },
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
