# Task formats

## Local four-block layout

```markdown
## 原始输入（原文，勿改）
<verbatim user input>

> 状态: ⬜ 待开始 (0/2)
> 分支: (待执行)
> 更新: 2026-07-20

- ⬜ 1. <task one verbatim text>
- ⬜ 2. <task two verbatim text>

## 说明与上下文（完成后补）
- 做了什么：
- 关键决策：
- 涉及文件：
- 验证结果：
- 风险与后续：
```

Allowed file states: `⬜ 待开始`, `🔄 进行中`, `⏸️ 阻塞`, `✅ 已完成`.

Allowed task markers: `⬜`, `🔄`, `⏸️`, `✅`.

The progress numerator equals the number of `✅` working task lines. The denominator equals all working task lines. Do not count lines inside the original-input block.

## File names

- Match only `todolist_<digits>.md` when allocating a number.
- Display at least two digits: `01`, `02`, ..., `99`, `100`.
- Ignore suffix variants such as `todolist_03_bug.md` for number allocation because they violate the canonical name.
- Never overwrite an existing path.

## NAS section

```markdown
## <project name>

> Plan：`<repository-relative plan path>`
> 执行源：`docs/todo/todolist_*.md`
> 同步策略：本地保存原始需求与执行细节，NAS 保存项目总览；显式同步，不做双向覆盖。

### P1 — <group>
- [ ] <task> 📅 YYYY-MM-DD 👤 <real operator>
  - 验收：<observable completion criterion>
```

Use P0 for immediate incidents, P1 for current delivery, P2 for planned improvements, and P3 for observation or optional work.

## Status mapping

| Local | NAS | Rule |
|---|---|---|
| `⬜ 待开始` | `- [ ]` | Keep active |
| `🔄 进行中` | `- [ ]` plus `🔄` | Include progress and branch |
| `⏸️ 阻塞` | `- [ ]` plus `⏸️` | Include cause and release condition |
| `✅ 已完成` | `- [x]` | Add completion date, then follow the vault archive convention |
