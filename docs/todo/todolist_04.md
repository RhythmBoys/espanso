## 原始输入（原文，勿改）
- 还是报配置文件夹必须是空,我的要求是可以不为空,或者可以导出再导入,方便迁移和备份 使用检索skill,查找符合最佳用户体验的方案，做文档架构设计

> 状态: ✅ 已实现 (3/3 阶段)
> 分支: `yxq-settings-portable-backup`
> 更新: 2026-08-15
> 设计: `docs/plans/2026-08-15-config-portable-backup-and-folder-intents-design.md`

- ✅ P0 信息架构：Open existing / Create new 分钮；`...` 只 adopt；Copy 降为 Advanced；错误文案互指
- ✅ P1 Export backup（`.espanso-backup.zip` + 清单）
- ✅ P2 Import backup（校验→预览→确认；默认新目录切换；手 zip 包装层探测）

## 说明与上下文

- **根因**：`Choose new directory...` 走复制路径，目标合法要求空；与 Open 入口文案混淆；缺文件型备份。
- **实现**：
  - `AdoptService` 只做 Open（空目录拒绝并指向 Create）。
  - `ScaffoldService` / `MigrationService` 错误文案互指 Open / Import。
  - 新增 `BackupService`：export / plan_import / execute_import / discard_import。
  - UI：Open / Create / Export / Import 主区；Copy 在 Advanced。
  - Import：先选 zip 再选空目标目录 → 摘要预览 → Restore and switch。
- **验证**：`cargo test -p espanso-settings --no-default-features` 35 项全过（含 backup 6 项）；clippy（ui + headless）零警告；`cargo check -p espanso-settings --features ui` 通过。
