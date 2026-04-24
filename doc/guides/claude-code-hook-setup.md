# Claude Code Hook 开发指引

本文档说明如何在本仓库本地启用 Claude Code 的 Rust 格式化辅助 hook。

## 背景

仓库已经提交了可复用的 hook 脚本实现，位于 [scripts/claude-hooks/](../../scripts/claude-hooks/)：

- [rust-fmt-pre-commit-wrapper.mjs](../../scripts/claude-hooks/rust-fmt-pre-commit-wrapper.mjs)
- [rust-fmt-stop-warning-wrapper.mjs](../../scripts/claude-hooks/rust-fmt-stop-warning-wrapper.mjs)
- [rust-fmt-pre-commit.mjs](../../scripts/claude-hooks/rust-fmt-pre-commit.mjs)
- [rust-fmt-stop-warning.mjs](../../scripts/claude-hooks/rust-fmt-stop-warning.mjs)
- [rust-fmt-lib.mjs](../../scripts/claude-hooks/rust-fmt-lib.mjs)

项目根下的 `.claude/` 默认被 `.gitignore` 忽略，因此是否启用、如何启用，仍由每位开发者在本地 `.claude/settings.local.json` 中自行决定。

## 推荐配置位置

建议把本地 Claude Code hook 配置写在 `.claude/settings.local.json` 中，而不是共享到仓库默认配置里。这样可以避免把个人开发偏好直接带给所有协作者。

## macOS / Linux 配置样例

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bun \"${CLAUDE_PROJECT_DIR}/scripts/claude-hooks/rust-fmt-pre-commit-wrapper.mjs\"",
            "if": "Bash(git commit*)"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bun \"${CLAUDE_PROJECT_DIR}/scripts/claude-hooks/rust-fmt-stop-warning-wrapper.mjs\""
          }
        ]
      }
    ]
  }
}
```

## Windows 配置样例

Windows 上推荐优先确认以下命令都能直接在 Claude hook 进程里被找到：

- `bun`
- `git`
- `rustfmt`

配置内容可与 macOS / Linux 保持一致，仍然写入 `.claude/settings.local.json`：

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bun \"${CLAUDE_PROJECT_DIR}/scripts/claude-hooks/rust-fmt-pre-commit-wrapper.mjs\"",
            "if": "Bash(git commit*)"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bun \"${CLAUDE_PROJECT_DIR}/scripts/claude-hooks/rust-fmt-stop-warning-wrapper.mjs\""
          }
        ]
      }
    ]
  }
}
```

## 行为说明

### PreToolUse

当你通过 Claude Code 调用 `git commit ...` 时：

- 如果本次提交既不包含 Rust 文件，也不包含会进入 Prettier 检查范围的前端 / 文档文件，hook 会直接跳过。
- 如果本次提交包含 `js`、`mjs`、`cjs`、`ts`、`svelte`、`md` 文件，hook 会对这些本次提交涉及且当前仍存在的文件执行 `prettier --write`。
- 如果本次提交包含 Rust 文件，hook 会对这些本次提交涉及且当前仍存在的 Rust 文件执行 `rustfmt`。
- 只要 Prettier 或 rustfmt 改写了文件，hook 就会阻止提交，并提示你检查 diff、重新暂存后再提交。
- 如果你使用了部分暂存（partial staging），Prettier / rustfmt 仍会直接改写工作区文件；启用前建议先确认这符合你的本地工作流预期。
- 如果 Prettier 或 rustfmt 执行失败，hook 也会阻止提交，并输出错误信息。

### Stop

当 Claude 完成一轮响应时：

- hook 会先判断当前工作区里是否存在本地改动的 Rust 文件，或存在会进入 Prettier 检查范围的前端 / 文档文件。
- 只要存在前端 / 文档类改动，hook 就会执行与 CI 一致的 `bun run format:check`。
- 只要存在 Rust 改动，hook 就会进一步执行 `cargo fmt --all --check` 做工作区级格式检查。
- 如果任一检查失败，hook 会给出提醒。
- 这个提醒不会阻塞后续操作。

## 使用建议

- 初次启用后，建议先在一个临时分支或临时 worktree 中做一次试跑，确认本机 `bun`、`git`、`rustfmt` 路径都正常。
- 如果你主要在 Windows 上开发，建议优先确认当前 Claude Code 使用的 shell 环境与 PATH 设置一致，避免交互式终端里可用、hook 进程里不可用的情况。
- 如果你修改了这些 hook 脚本，优先修改仓库内的 [scripts/claude-hooks/](../../scripts/claude-hooks/) 版本，再按需同步你的本地 `.claude` 配置引用。
