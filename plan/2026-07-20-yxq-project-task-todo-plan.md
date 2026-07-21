# yxq-project-task-todo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `yxq-project-task-todo` skill，以本地 `docs/todo/todolist_*.md` 管理可执行任务，以 NAS Obsidian 根 `todo.md` 管理跨项目总览，并保证两端可追踪、可恢复、不覆盖既有内容。

**Architecture:** 采用“双层单向汇总”结构：Git 中的 `todolist_*.md` 保存逐字原始需求、执行状态、分支和完成上下文；NAS `todo.md` 保存项目级优先级、负责人、验收标准和本地任务引用。Skill 负责创建、扫描、执行和汇总，NAS 更新始终执行“读取—解析—合并—备份—写回—回读校验”。

**Tech Stack:** Markdown、YAML frontmatter、PowerShell 7、Git worktree、OpenSSH、Synology NAS、Obsidian。

## Global Constraints

- Skill 名称固定为 `yxq-project-task-todo`，目录固定为 `.claude/skills/yxq-project-task-todo/`。
- 本地执行队列固定为 `docs/todo/todolist_<编号>.md`；编号取现有最大数字加一，至少两位零填充，不回填空号，不覆盖旧文件。
- 本地任务文件采用四块布局：原始输入、状态块、任务清单、说明与上下文；原始输入逐字保留。
- 一个 `todolist_*.md` 对应一个独立 worktree 分支；多个文件顺序执行，不嵌套 worktree。
- NAS 唯一路径固定为 `/volume2/Sharefile/Obsidian-KB/AltitudeCraft/todo.md`；禁止在项目子目录创建第二个 `todo.md`。
- NAS 多项目以 H2 分区；本项目使用 `## espanso（yxq-project-task-todo）`。
- NAS 按 P0–P3 分组；完成项移动到既有归档机制，不在活跃区长期保留。
- NAS 写入者固定记录为 `yxq`，`source` 记录为 `Codex`，日期使用 Asia/Shanghai 的 `YYYY-MM-DD`。
- NAS 更新必须保留所有未知 frontmatter 字段、其他项目分区、Wiki 链接和人工注释。
- NAS 认证失败、旧文件为空、解析失败、目标分区重复或回读不一致时立即停止，不覆盖远端文件。
- 不引入数据库、Web UI、定时任务或双向自动同步；首版只实现显式触发，遵循 YAGNI。

---

## 1. 文件结构

| 路径 | 操作 | 单一职责 |
|---|---|---|
| `.claude/skills/yxq-project-task-todo/SKILL.md` | 新建 | 定义触发条件、状态机、创建/执行/同步工作流和安全门禁 |
| `.claude/skills/yxq-project-task-todo/scripts/todo.ps1` | 新建 | 扫描编号、创建任务文件、解析状态、生成 NAS 项目分区 |
| `.claude/skills/yxq-project-task-todo/scripts/sync-nas-todo.ps1` | 新建 | 备份并合并 NAS `todo.md`，执行回读校验 |
| `.claude/skills/yxq-project-task-todo/references/formats.md` | 新建 | 保存本地四块布局、NAS 分区和状态映射的规范示例 |
| `.claude/skills/yxq-project-task-todo/tests/todo.Tests.ps1` | 新建 | 覆盖编号、原文保护、解析、幂等和失败保护 |
| `.claude/skills/yxq-project-task-todo/tests/sync-nas-todo.Tests.ps1` | 新建 | 使用本地 fixture 验证保留式合并，不连接真实 NAS |
| `docs/todo/` | 按需创建 | 存放 Git 可审计的执行任务文件 |
| `plan/nas-obsidian-todo-section.md` | 新建 | 保存本次待合并的 NAS 项目分区，便于审核与恢复 |

## 2. 数据契约

### 2.1 本地任务文件

```markdown
## 原始输入（原文，勿改）
<用户原文>

> 状态: ⬜ 待开始 (0/2)
> 分支: (待执行)
> 更新: 2026-07-20

- ⬜ 1. <任务一原文>
- ⬜ 2. <任务二原文>

## 说明与上下文（完成后补）
- 做了什么：
- 关键决策：
- 涉及文件：
- 验证结果：
- 风险与后续：
```

合法文件状态为 `⬜ 待开始`、`🔄 进行中`、`⏸️ 阻塞`、`✅ 已完成`。合法任务状态为 `⬜`、`🔄`、`⏸️`、`✅`。文件进度必须等于已完成任务数除以任务总数。

### 2.2 NAS 项目分区

```markdown
## espanso（yxq-project-task-todo）

> Plan：`plan/2026-07-20-yxq-project-task-todo-plan.md`
> 执行源：`docs/todo/todolist_*.md`
> 同步策略：本地保存执行细节，NAS 保存项目总览；显式同步，不做双向覆盖。

### P1 — Skill 创建与验证
- [ ] 创建并验证 `yxq-project-task-todo` skill 📅 2026-07-20 👤 yxq
  - 验收：编号、原文保护、状态推进、幂等合并和失败保护测试全部通过。
```

### 2.3 状态映射

| 本地状态 | NAS 状态 | 行为 |
|---|---|---|
| `⬜ 待开始` | `- [ ]` | 留在活跃分区 |
| `🔄 进行中` | `- [ ]` + `🔄` | 更新进度和分支 |
| `⏸️ 阻塞` | `- [ ]` + `⏸️` | 必须附阻塞原因和解除条件 |
| `✅ 已完成` | `- [x]` | 写入完成日期，随后按 NAS 现有归档机制迁移 |

## 3. 用户体验与错误处理

- “新增任务”默认只创建下一个 `todolist_NN.md` 并显示文件路径、任务数和下一步；只有用户明确说“新增并执行”才创建 worktree。
- “查看待办”按 P0–P3、状态和文件编号输出摘要，不读取无关源码。
- “执行待办”开始前展示目标文件、分支名和任务数；每完成一项立即更新文件状态。
- “同步 NAS”先显示 dry-run 差异摘要；只有目标分区唯一且远端备份成功才写回。
- 网络或 SSH 失败时保留本地结果，提示重试命令，不把本地任务回滚为未完成。
- 合并冲突时生成候选分区文件，不猜测人工内容的归属。

## 4. 实施任务

### Task 1: 建立 skill 骨架和格式契约

**Files:**
- Create: `.claude/skills/yxq-project-task-todo/SKILL.md`
- Create: `.claude/skills/yxq-project-task-todo/references/formats.md`

**Interfaces:**
- Consumes: 本计划第 2 节的数据契约。
- Produces: 后续脚本必须遵守的路径、状态、触发词和安全门禁。

- [ ] **Step 1:** 用 skill-creator 的 `init_skill.py` 初始化目录，仅创建 `scripts,references` 资源目录和 `agents/openai.yaml`。
- [ ] **Step 2:** 在 `SKILL.md` frontmatter 中仅保留 `name` 和 `description`；description 同时覆盖“新增任务、执行队列、查看进度、同步 NAS”中英文触发语。
- [ ] **Step 3:** 将本地四块布局、NAS 分区和状态映射写入 `references/formats.md`，`SKILL.md` 只保留核心流程并按需引用该文件。
- [ ] **Step 4:** 运行 `quick_validate.py .claude/skills/yxq-project-task-todo`；预期退出码为 0，且无 frontmatter 或命名错误。
- [ ] **Step 5:** 提交 `feat(skill): scaffold yxq project task todo`。

### Task 2: 用 Pester 锁定本地任务行为

**Files:**
- Create: `.claude/skills/yxq-project-task-todo/tests/todo.Tests.ps1`
- Create: `.claude/skills/yxq-project-task-todo/scripts/todo.ps1`

**Interfaces:**
- Produces: `Get-NextTodoNumber`, `New-ProjectTodo`, `Get-ProjectTodoState`, `Set-ProjectTodoTaskState`。

- [ ] **Step 1:** 写失败测试：空目录返回 `01`；已有 `01,03,100` 返回 `101`；非标准文件名不参与编号。
- [ ] **Step 2:** 写失败测试：创建文件后原始输入逐字相等，任务编号连续，状态为 `⬜ 待开始 (0/N)`，再次使用同一编号必须失败。
- [ ] **Step 3:** 写失败测试：状态推进只修改工作清单和状态块，原始输入块的 SHA-256 前后相同。
- [ ] **Step 4:** 运行 `Invoke-Pester .../todo.Tests.ps1`；预期测试失败，原因是函数尚未实现。
- [ ] **Step 5:** 在 `todo.ps1` 实现四个函数；所有写操作使用 `-LiteralPath`、UTF-8 无 BOM 和显式错误终止。
- [ ] **Step 6:** 再次运行 Pester；预期全部通过。
- [ ] **Step 7:** 提交 `feat(skill): add local todolist workflow`。

### Task 3: 实现 NAS 分区生成和保留式合并

**Files:**
- Create: `.claude/skills/yxq-project-task-todo/scripts/sync-nas-todo.ps1`
- Create: `.claude/skills/yxq-project-task-todo/tests/sync-nas-todo.Tests.ps1`

**Interfaces:**
- Consumes: `Get-ProjectTodoState` 的结构化结果。
- Produces: `New-NasTodoSection`、`Merge-NasTodoSection`、`Sync-NasTodo`。

- [ ] **Step 1:** 创建包含 frontmatter、其他项目 H2、目标分区和 Wiki 链接的 fixture。
- [ ] **Step 2:** 写失败测试：首次合并只新增一个目标 H2；二次合并字节级幂等；其他项目内容和 Wiki 链接保持不变。
- [ ] **Step 3:** 写失败测试：空远端、缺少 frontmatter、重复目标 H2、SSH 非零退出码和回读哈希不一致都必须停止写入。
- [ ] **Step 4:** 实现纯文本 `Merge-NasTodoSection`，用精确 H2 边界替换目标分区；不得用模糊正则替换相似项目名。
- [ ] **Step 5:** 实现 `Sync-NasTodo -WhatIf`：读取远端、生成差异摘要、创建带时间戳备份、上传临时文件、远端原子替换、回读 SHA-256 校验。
- [ ] **Step 6:** 运行 Pester；预期全部通过，测试过程不连接真实 NAS。
- [ ] **Step 7:** 提交 `feat(skill): add safe NAS todo synchronization`。

### Task 4: 编写完整的 Skill 编排说明

**Files:**
- Modify: `.claude/skills/yxq-project-task-todo/SKILL.md`
- Modify: `.claude/skills/yxq-project-task-todo/agents/openai.yaml`

**Interfaces:**
- Consumes: Tasks 2–3 的函数和错误语义。
- Produces: Agent 可执行的新增、查看、执行、同步四条路径。

- [ ] **Step 1:** 明确新增路径：扫描最大编号、写入原始输入、默认不执行、输出下一步。
- [ ] **Step 2:** 明确执行路径：过滤已完成文件、一文件一 worktree、按编号推进、验证后补齐上下文、退出 worktree 后再处理下一文件。
- [ ] **Step 3:** 明确同步路径：默认 `-WhatIf`、展示差异、确认后备份与写回；任何门禁失败都只保留候选文件。
- [ ] **Step 4:** 记录与 `yxq-worktree-branch`、`deep-research`、`brainstorming`、`harness-sdd`、`verification-before-completion` 的触发关系，不复制这些技能的内部内容。
- [ ] **Step 5:** 重新生成 `agents/openai.yaml`，默认提示词说明“保留原文并安全同步 NAS”。
- [ ] **Step 6:** 运行 `quick_validate.py` 和全部 Pester 测试；预期全部通过。
- [ ] **Step 7:** 提交 `docs(skill): finalize task todo orchestration`。

### Task 5: 端到端验收

**Files:**
- Create during test: 临时目录中的 `docs/todo/todolist_01.md`、`02.md`、`04.md`
- Verify: `/volume2/Sharefile/Obsidian-KB/AltitudeCraft/todo.md`

**Interfaces:**
- Consumes: 完整 skill 和脚本。
- Produces: 可复现的验收记录。

- [ ] **Step 1:** 在临时 Git 仓库模拟“新增任务”，确认产生 `todolist_01.md` 且不会立即开分支。
- [ ] **Step 2:** 模拟编号存在空洞，确认下一编号为最大值加一。
- [ ] **Step 3:** 模拟任务从待开始到完成，确认原始输入 SHA-256 不变、进度为 `N/N`、完成上下文齐全。
- [ ] **Step 4:** 对 NAS 执行 `-WhatIf`，人工核对仅新增或替换目标 H2，其他分区无差异。
- [ ] **Step 5:** 执行真实同步，确认备份存在、远端 frontmatter 的 `updated_at` 为当天、`updated_by` 为 `yxq`、回读哈希一致。
- [ ] **Step 6:** 模拟网络中断，确认远端原文件仍可读，本地候选文件仍存在。
- [ ] **Step 7:** 运行 `git diff --check`、skill validator 和 Pester；预期全部通过。
- [ ] **Step 8:** 提交 `test(skill): verify project task todo workflow`。

## 5. 完成定义

- 新增、查看、执行和同步四类触发语均能命中 skill。
- `todolist_*.md` 编号严格递增，原始输入可用哈希证明未被修改。
- 每个任务文件只对应一个 worktree，多个任务文件不并行、不嵌套。
- NAS 同步幂等，只修改目标分区和允许更新的 frontmatter 字段。
- 远端写入前存在备份，写入后通过哈希回读验证。
- 所有 Pester 测试、skill validator 和 `git diff --check` 通过。
- 文档不含未定义占位符，实施者无需补做设计决策。

## 6. 回滚方案

1. 停止继续同步，保留失败时生成的本地候选文件。
2. 从远端同目录的时间戳备份恢复 `todo.md`。
3. 回读文件并比较 SHA-256；不一致时停止自动操作并人工审查。
4. 本地 skill 代码通过独立提交回滚，不删除既有 `todolist_*.md`。
