---
name: yxq-project-task-todo
description: Manage a project's Git-tracked todo queue and safely summarize it to the shared NAS Obsidian todo.md. Use when the user asks to 新增任务、加个任务、记个待办、新建 todolist、查看待办、处理待办、执行任务清单、完成所有任务、同步 NAS 待办、更新 Obsidian todo, or says add task, new todo, run/process the todo queue, show todo progress, or sync project todos to NAS. Also use when docs/todo/todolist_*.md files need status updates or execution.
---

# YXQ Project Task Todo

Use Git-tracked `docs/todo/todolist_*.md` files as the execution source and the shared NAS Obsidian `todo.md` as the cross-project summary.

## Non-negotiable rules

1. Preserve the original input verbatim. Add status information only to the working copy below it.
2. Allocate the next file number as `max(existing numbers) + 1`; never fill gaps or overwrite a file.
3. Process one todo file per worktree. Exit that worktree before opening another.
4. Treat local files as execution detail and NAS as summary. Do not implement automatic two-way synchronization.
5. Read and validate the complete NAS file before writing. Preserve other projects, unknown frontmatter fields, Wiki links, and comments.
6. Stop without replacing NAS `todo.md` when authentication, parsing, backup, upload, or hash verification fails.

Read [references/formats.md](references/formats.md) before creating, annotating, or synchronizing tasks.

## Create a task file

1. Load `scripts/todo.ps1`.
2. Pass the user's verbatim input and normalized task lines to `New-ProjectTodo`.
3. Report the created path, task count, and `⬜ 待开始` state.
4. Do not execute it unless the user explicitly requests creation and execution in the same message.

```powershell
. .claude/skills/yxq-project-task-todo/scripts/todo.ps1
New-ProjectTodo -TodoDirectory docs/todo -OriginalInput $original -Tasks $tasks
```

## Show progress

Scan only `docs/todo/todolist_*.md`. Use `Get-ProjectTodoState` for each file, then group the result by file state. Do not open a worktree for a read-only progress request.

## Execute the queue

1. Sort task files by their numeric component and skip `✅ 已完成` files.
2. Read the entire current file and resolve any referenced skills before implementation.
3. Use `yxq-worktree-branch` when available; otherwise use the platform's approved worktree workflow.
4. Mark the file and current task `🔄`, implement the task, and verify the result.
5. Mark a verified task `✅`; on a genuine blocker mark it `⏸️` and record the reason and release condition.
6. After all tasks pass verification, set the file to `✅ 已完成`, complete the context section, finish the branch, and exit the worktree.
7. Start the next file only after the previous worktree has exited.

Invoke related skills when applicable: `deep-research` for explicit research, `superpowers:brainstorming` for design choices, `harness-sdd` for broad or high-risk changes, and `superpowers:verification-before-completion` before completion claims.

## Synchronize NAS

Use the reviewed project section as input. Run dry-run first.

```powershell
. .claude/skills/yxq-project-task-todo/scripts/sync-nas-todo.ps1
Sync-NasTodo -SectionPath plan/nas-obsidian-todo-section.md -WhatIf
```

After reviewing the candidate diff, rerun without `-WhatIf`. The script must create a timestamped remote backup, upload to a temporary path, replace atomically, and compare the remote SHA-256 with the local candidate.

The canonical remote path is `/volume2/Sharefile/Obsidian-KB/AltitudeCraft/todo.md`. Use `peter-admin@192.168.1.167` unless the project supplies a more specific verified SSH configuration. Record `updated_by` using the real operator name supplied by the user.

## Failure behavior

- Keep the local task state when NAS is unavailable.
- Keep the generated candidate when synchronization fails.
- Never delete a task file or remote backup automatically.
- When the target H2 appears more than once, stop and request human review.
- When a completed item needs removal from the active NAS area, follow the existing vault archive convention instead of inventing a new archive.
