# Bun Migration Plan

> 将当前仓库从 pnpm 迁移到 Bun 的独立实施文档。

## 迁移进度

**提交**: `4f510fb` (2026-04-20)

### 已完成 ✅

- [x] 阶段 1: 生成并验证 Bun 锁文件，删除 pnpm 锁文件
- [x] 阶段 2: 项目脚本改成 Bun，固定单一 Bun 版本策略
- [x] 阶段 3: Tauri 前端构建钩子切到 Bun
- [x] 阶段 4-A: CI workflow 迁移
- [x] 阶段 4-B: Release workflow 迁移
- [x] 阶段 5: 更新 README 与其余文档/模板
- [x] 阶段 7 (部分): 收尾验证与残留清理

### 待验证 ⏳

- [ ] 本地 `bun run tauri:dev` 验证
- [ ] 本地 `bun run tauri:build` 验证
- [ ] CI `windows-latest` 实际通过
- [ ] Release workflow 三平台实际通过

---

## 背景

当前仓库刚刚从 npm 迁移到 pnpm（提交 `3a27176`），`pnpm-lock.yaml` 是唯一 JS 锁文件。下一步目标是进一步切到 Bun；这次变更的核心不是业务代码，而是把前端依赖安装、脚本执行、Tauri 前端构建钩子、CI 和 release workflow 统一到 Bun。

多轮复审后，当前已确认这次迁移的主要风险集中在四处：

- `windows-latest` 上的 Bun 安装与 `--frozen-lockfile` 是否稳定
- `tauri-apps/tauri-action` 中把 `pnpm exec tauri` 改成 Bun 等价调用后的兼容性
- 锁文件从 `pnpm-lock.yaml` 迁移到 `bun.lock` 后，本地与 CI / release runner 的一致性
- Bun 对依赖生命周期脚本的处理与 pnpm 不同，是否需要补充 `trustedDependencies`

## 当前状态

已确认的现状（基于当前仓库状态）：

- `package.json` 脚本内嵌 `pnpm run ...` / `pnpm exec ...`，`packageManager` 声明 `pnpm@10.33.0`
- `src-tauri/tauri.conf.json` 的 `beforeBuildCommand` / `beforeDevCommand` 调用 `pnpm run ...`
- `.github/workflows/ci.yml` / `distribute.yml` 使用 `pnpm/action-setup@v4`、pnpm cache、`pnpm install --frozen-lockfile`、`pnpm run check`
- `distribute.yml` 的 `tauriScript` 用 `pnpm exec tauri`
- `distribute.yml` 中还有 `node -e` 的版本改写步骤，因此 release workflow 当前仍真实依赖 Node
- README、CLAUDE.md、前端指南、发布流程、PR 模板均以 pnpm 为默认命令
- `.gitignore` 目前忽略 `bun.lockb`，但不忽略 `bun.lock`；现代 Bun 默认锁文件是 `bun.lock`
- 仓库当前没有 Bun 专用配置文件，也没有任何 `trustedDependencies` 声明
- `package-lock.json` 已在上一轮 pnpm 收敛中删除，这次迁移后也必须保持不存在

## 迁移原则

这次 Bun 迁移采用**更保守的分阶段方案**：

1. 先稳定本地与 CI
2. 再切 release workflow
3. 最后统一文档并做 repo 级残留清理

## 实施方案

### 1. 先生成并验证 Bun 锁文件，再删除 pnpm 锁文件

目标：把锁文件真相来源从 `pnpm-lock.yaml` 安全迁移到 `bun.lock`。

步骤：

- **重要**：Bun 不支持从 `pnpm-lock.yaml` 导入锁文件。`bun install` 会直接从 `package.json` 解析依赖并生成新的 `bun.lock`
- 在仓库根目录运行 `bun install`，生成 `bun.lock`
- 手动 review `bun.lock` diff，确认没有意外的依赖版本变更
- 在删除 `pnpm-lock.yaml` 之前，先验证：
  - 本地 `bun install --frozen-lockfile` 可通过
  - 统一使用 `bun install --frozen-lockfile`（不使用 `bun ci`，避免混淆）
  - 在 CI 等价环境里至少验证一次 `windows-latest` 上的 `bun install --frozen-lockfile`
- 验证通过后，再删除 `pnpm-lock.yaml`
- `bun.lock` 必须提交进仓库；`bun.lockb` 仍保持忽略
- `package-lock.json` 必须继续保持删除状态，不得重新引入
- 检查并清理 `.npmrc` 中的 pnpm 相关配置（如 `save-exact=true`）

边界：

- 不改依赖版本策略本身，只换包管理器
- 不引入额外 workspace 结构

### 2. 把项目脚本改成 Bun，并固定单一 Bun 版本策略

目标：所有入口命令本地运行不再隐式依赖 pnpm，且本地 / CI 使用同一 Bun 版本策略。

调整内容：

- 在 `package.json` 中把 `packageManager` 从 `pnpm@10.33.0` 改为固定的 `bun@<exact-version>`
- 把 `package.json.packageManager` 作为 Bun 版本的**唯一真相源**
- 在 `.github/workflows/ci.yml` 和 `.github/workflows/distribute.yml` 中，`oven-sh/setup-bun@v2` 应默认读取 `package.json`，或显式使用 `bun-version-file: package.json`；不要在 workflow 再手写第二份 Bun 版本号
- 把脚本中的 `pnpm run ...` / `pnpm exec svelte-check` 改为 Bun 等价：
  - `pnpm run check:types && pnpm run check:svelte && pnpm run check:build` → `bun run check:types && bun run check:svelte && bun run check:build`
  - `pnpm exec svelte-check` → `bunx svelte-check`
- 保留 `dev` / `build` / `tauri:*` / `check:cargo` 脚本名不变，只替换执行层

风险控制：

- 升级 Bun 版本时，只改 `package.json.packageManager`，CI / release 跟随它，不再维护第二个版本锚点
- 首次 `bun install` 后，要显式检查依赖生命周期脚本是否触发异常
- **trustedDependencies 触发条件**：仅当某依赖的安装脚本（`postinstall`、`prepublish` 等）在依赖安装阶段必须执行，否则包会无法正常工作时才需要。Svelte 生态一般不需要，但 Vite 插件链中偶有例外
- 若需要引入 `trustedDependencies`，要把原因写进迁移 PR 或相关文档，避免后续维护者不知道为什么必须信任这些依赖

### 3. 把 Tauri 前端构建钩子切到 Bun

目标：桌面开发与打包流程不再调用 pnpm。

调整内容：

- 在 `src-tauri/tauri.conf.json` 中：
  - `beforeBuildCommand`: `pnpm run build` → `bun run build`
  - `beforeDevCommand`: `pnpm run build && pnpm run dev` → `bun run build && bun run dev`

风险控制：

- 这一步只改钩子命令，不改现有 dev/build 编排语义
- 需要在本地至少验证一次 `bun run tauri:dev`，确认 dev server 和 Tauri 窗口都能正常起来

### 4. 先保守迁移 CI，再迁移 release workflow

目标：先把容易回滚的检查链路切到 Bun，再处理更高风险的发版链路。

#### Phase A：迁移 `.github/workflows/ci.yml`

- 移除 `pnpm/action-setup@v4`
- **保守做法**：先保留 `actions/setup-node@v6`，但去掉 pnpm cache 相关配置；再额外添加 `oven-sh/setup-bun@v2`
- `oven-sh/setup-bun@v2` 应读取 `package.json.packageManager`，而不是在 workflow 里再维护一个独立 Bun 版本
- 不使用 `actions/setup-node` 的 `cache: bun`，因为它不支持 Bun
- 把 `pnpm install --frozen-lockfile` 改为 `bun install --frozen-lockfile`
- 把 `pnpm run check` 改为 `bun run check`
- **预热验证**：在正式 PR 前，通过手动触发 workflow 或用 `act` 模拟预跑 `windows-latest` 上的 `bun install --frozen-lockfile`
- 先让 PR CI 在 Windows 上连续通过，再继续动 release workflow

#### Phase B：迁移 `.github/workflows/distribute.yml`

- 保留 `actions/setup-node@v6`，因为 workflow 里现有的 `node -e` 版本改写步骤确实依赖 Node
  - **版本改写来源说明**：当前 `node -e` 脚本从 `package.json` 和 `src-tauri/tauri.conf.json` 读取版本并改写。这个步骤仍需 Node，暂不替换为 `bun -e`
- 添加 `oven-sh/setup-bun@v2`，并读取 `package.json.packageManager`，不要在 workflow 里额外写死 Bun 版本
- 移除 `pnpm/action-setup@v4`
- 把 `pnpm install --frozen-lockfile` 改为 `bun install --frozen-lockfile`
- 把 `tauriScript` 从 `pnpm exec tauri` 改为**优先验证后的 Bun 等价调用**
  - **验证流程**：本地先运行 `bun tauri build --help`，如果报错尝试 `bunx @tauri-apps/cli build --help`，以退出码判断可用性
  - 默认优先候选是 `bun tauri` 或 `bun run tauri`，尽量沿用项目已有 script / CLI 入口语义
  - `bunx tauri` 作为回退候选，而不是默认候选
  - 在真正修改 workflow 前，要先通过本地或等价环境验证 `bun tauri` / `bun run tauri` / `bunx tauri` 哪个和 `tauri-action` 更兼容，再定最终写法

风险控制：

- CI 与 release 分两步切，避免一次性同时破坏 PR 检查和发版流程
- release workflow 修改前，必须先验证 Bun 版 Tauri CLI 调用路径，而不是直接假设 `bunx tauri` 可用
- 在真正切 release workflow 前，先对 `windows-latest`、`macos-15`、`macos-15-intel` 各做至少一次 `bun install --frozen-lockfile` + `bun run build` smoke，确认锁文件和基础构建在 release runner 上都能成立

### 5. 更新 README 与其余文档/模板

目标：把 “bun-only” 写成仓库显式约定，并同步所有入口文档。

需要更新的文件：

- `README.md`
- `CLAUDE.md`
- `doc/FRONTEND_GUIDE.md`
- `doc/RELEASE_PROCESS.md`
- `.github/pull_request_template.md`

文档要点：

- README 的本地开发命令改为 Bun，并明确 `bun.lock` 是唯一 JS 锁文件，且需要提交
- README 中移除 pnpm / Corepack 指导，改成 Bun 安装方式
- README 要明确写清：Bun 是唯一 JS 包管理器；Node 在本地是否仍是硬前置要给出单一口径，不要含糊
- `CLAUDE.md` 的常用命令全部改成 Bun，避免后续 agent 继续生成 pnpm 指令
- `doc/FRONTEND_GUIDE.md` 和 `doc/RELEASE_PROCESS.md` 中涉及检查/构建的命令同步替换为 Bun
- PR 模板里的测试清单改为 `bun run check`
- 文档里涉及 Bun 版本时，要与 `package.json` 和 workflow 中的 Bun 版本策略保持一致

### 6. 加入回滚方案

目标：如果 Bun 迁移在 CI 或 release 上失败，可以快速回退到 pnpm。

回滚策略：

- **在迁移前创建具名分支或标签作为回滚锚点**，例如 `rollback-pnpm-before-bun-migration`，避免回滚时找不准锚点
- 如果 CI 阶段失败：
  - 回退 workflow、`package.json`、`tauri.conf.json`、文档和锁文件改动
  - 恢复 `pnpm-lock.yaml`
  - 重新执行 `pnpm install` 确认 lockfile 与依赖树一致
- 如果 release workflow 阶段失败：
  - 优先只回退 release workflow 改动，而不是把整个 CI 一并回退
  - 避免在发版链路失败时同时丢失已验证通过的 CI 迁移成果

### 7. 收尾验证与残留清理

目标：确保迁移后本地、CI、release 和文档全部一致，不再残留 pnpm 痕迹。

收尾动作：

- repo 级搜索时，不只搜 `pnpm `，而是搜更宽的 `pnpm`，避免漏掉：
  - `pnpm@10.33.0`
  - `pnpm/action-setup`
  - `cache: pnpm`
  - `pnpm-lock.yaml`
- 也要搜索 `corepack`，避免 README 或协作文档残留旧的 pnpm 启用方式
- 检查并清理 `.npmrc` 文件中的 pnpm 相关配置
- 也要确认 `package-lock.json` 没有重新出现
- 对变化最大的调用路径做专项验证：
  - `bun run check:svelte`
  - `bun run check`
  - `bun run build`
  - `bun run tauri:dev`
  - Bun 版 Tauri CLI 调用（`bun tauri` / `bun run tauri` / 最终选定方案）
- **强制前置**：在切 release workflow 前，必须先做一次等价 dry run / 测试发版验证，不再只是”有条件再做”

## 关键文件

### Runtime / tooling

- `package.json`
- `.gitignore`
- `bun.lock`（新建，需提交）
- `bun.lockb`（继续忽略）
- `pnpm-lock.yaml`（删除）
- `package-lock.json`（必须继续保持删除）
- `src-tauri/tauri.conf.json`
- 如需要，`bunfig.toml` 或 `package.json` 中的 `trustedDependencies`

### GitHub workflows / templates

- `.github/workflows/ci.yml`
- `.github/workflows/distribute.yml`
- `.github/pull_request_template.md`

### Docs

- `README.md`
- `CLAUDE.md`
- `doc/FRONTEND_GUIDE.md`
- `doc/RELEASE_PROCESS.md`

## 验证清单

### Lockfile / policy

- 仓库只保留 `bun.lock` 作为 JS 锁文件，且已提交
- `bun.lockb` 没有被误生成或提交
- `pnpm-lock.yaml` 已删除且不再出现在 diff 中
- `package-lock.json` 没有重新出现
- `package.json` 的 `packageManager` 与 workflow 中的 Bun 版本完全一致
- Bun 版本的唯一真相源是 `package.json.packageManager`；workflow 只读取它，不再手写第二份版本号

### Local

- `bun install --frozen-lockfile`
- 首次安装后已检查依赖生命周期脚本行为；如需 `trustedDependencies`，已补齐并复验
- `bun run check:svelte`
- `bun run check`
- `bun run build`
- `bun run tauri:dev`（至少验证命令链路可启动）
- 如环境允许，再验证一次 `bun run tauri:build`

### CI

- `ci.yml` 不再引用 pnpm setup、pnpm cache、`pnpm-lock.yaml`、`pnpm install`、`pnpm run check`
- `ci.yml` 在 `windows-latest` 上可稳定通过
- 在确认 Bun 路径稳定前，`ci.yml` 保留 `actions/setup-node@v6` 作为更保守的兼容层
- 在切 release workflow 前，至少已在 `macos-15` 与 `macos-15-intel` 上做过 `bun install --frozen-lockfile` + `bun run build` smoke

### Release workflow

- `distribute.yml` 不再引用 pnpm setup、pnpm cache、`pnpm-lock.yaml`、`pnpm install`、`pnpm exec tauri`
- `distribute.yml` 同时保留 `actions/setup-node@v6`（用于 `node -e` 版本改写）和 `oven-sh/setup-bun@v2`
- `tauriScript` 的最终 Bun 写法已经在等价环境验证过，而不是直接假设可用；优先验证 `bun tauri` / `bun run tauri`
- **强制前置**：三平台（macOS Intel / Apple Silicon / Windows）都必须至少做过一次 release 路径 smoke 或等价验证，不再以”如条件允许”为前提

### Docs consistency

- README、CLAUDE、前端指南、发布流程、PR 模板中的 pnpm 指令全部切到 Bun
- README 已明确说明仓库使用 Bun，`bun.lock` 需要提交，不再维护 pnpm 锁文件
- 文档中的 Bun 版本说明与 `package.json` / workflow 保持一致
- README 和协作文档已明确 Node 在本地是否仍为硬前置，避免同一仓库出现两套口径

### Final repo sweep

- repo 内不再有面向当前流程的 `pnpm` / `pnpm-lock.yaml` / `corepack` 残留引用
- 迁移不会改变现有业务功能，只改变包管理器与构建/文档入口
