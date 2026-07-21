## espanso（yxq-project-task-todo）

> Plan：`plan/2026-07-20-yxq-project-task-todo-plan.md`
> 参考 Skill：`07_ecom/.claude/skills/yxq-task-todo/SKILL.md`
> 执行源：`docs/todo/todolist_*.md`
> 同步策略：本地保存原始需求与执行细节，NAS 保存项目总览；显式同步，不做双向覆盖。

### P1 — Skill 创建与核心能力

- [x] 创建 `.claude/skills/yxq-project-task-todo/`，补齐 `SKILL.md`、脚本、格式参考和 UI metadata 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：skill validator 通过；触发描述覆盖新增、查看、执行和 NAS 同步。
- [x] 实现 `todolist_*.md` 严格递增编号与四块布局 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：空目录从 `01` 开始；存在空号仍取最大值加一；原始输入逐字保留。
- [x] 实现本地任务状态机和一文件一 worktree 编排 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：状态按 `待开始→进行中→完成/阻塞` 推进；多个文件顺序执行且不嵌套。

### P1 — NAS 安全同步

- [x] 实现 Obsidian `todo.md` 保留式合并 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：只新增或替换 `## espanso（yxq-project-task-todo）`；其他项目、frontmatter、Wiki 链接和人工注释保持不变。
- [x] 增加 dry-run、远端备份、临时文件原子替换和 SHA-256 回读校验 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：认证失败、空远端、重复分区、解析失败或哈希不一致时不覆盖远端。

### P2 — 自动化验证与交付

- [x] 为编号、原文保护、状态进度、幂等合并和失败保护编写 Pester 测试 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：测试不访问真实 NAS，全部通过后才能运行真实同步。
- [x] 完成端到端验收和文档自检 📅 2026-07-20 ✅ 2026-07-20 👤 yxq
  - 验收：skill validator、Pester、`git diff --check` 全部通过；NAS 回读内容与候选分区一致。

### P3 — 后续观察项

- [ ] 连续使用三个任务周期后评估是否需要自动归档或定时同步 📅 2026-08-20 👤 yxq
  - 启动条件：出现两次以上人工漏同步；否则保持显式同步，避免不必要复杂度。
