## 原始输入（原文，勿改）
- 1. 增加setting菜单,点击后,增加新ui,选项卡模型,设置/配置
- 2. 设置界面是可以选择配置文件可以生成的自定义目录下,可以迁移原配置到自定义的目录下
- 3. 配置界面可以增加输入可替换的文本和真正替换后的长文本,请使用搜索的Skill，深入全面的搜索符合最佳实践，良好的用户体验方案

> 状态: ✅ 已完成 (3/3)
> 分支: `yxq-espanso-settings-ui`
> 更新: 2026-07-21
> Plan: `plan/2026-07-20-cross-platform-settings-config-editor-plan.md`
> 规划状态: ✅ 已确认（Slint 三端一致方案）

- ✅ 1. 增加setting菜单,点击后,增加新ui,选项卡模型,设置/配置
- ✅ 2. 设置界面是可以选择配置文件可以生成的自定义目录下,可以迁移原配置到自定义的目录下
- ✅ 3. 配置界面可以增加输入可替换的文本和真正替换后的长文本,请使用搜索的Skill，深入全面的搜索符合最佳实践，良好的用户体验方案

## 说明与上下文（完成后补）
- 做了什么：新增 `espanso-settings` crate 和 Slint 双选项卡窗口；打通托盘 `Settings…`、隐藏 CLI、worker IPC、单实例聚焦；实现配置路径优先级、原子持久化、迁移预检/复制/验证/切换，以及 `match/ui.yml` 的搜索、新增、编辑、删除、撤销、备份和原子保存。
- 关键决策：Slint 1.17.1 + rfd 0.17.2，三端复用同一 UI；迁移永不删除旧目录；首版只写 `match/ui.yml`；候选规则先在隔离副本中做整套配置验证，并阻止与其他 YAML 的 trigger 冲突。
- 涉及文件：`espanso-settings/`、`espanso/` 的 CLI/路径/IPC、`espanso-engine/` 的菜单与分发、workspace Cargo 文件及 `.github/workflows/ci.yml`；精确清单见 Plan 与 `git diff --stat`。
- 验证结果：Rust 1.97 Linux ARM64 Docker 中，CI 等价的 workspace Clippy、格式检查和 workspace 测试通过；Settings `cargo check` 与 16 项核心测试通过；主程序 Settings IPC 测试通过。
- 风险与后续：Windows/macOS/Linux CI 已配置，但三端真实窗口、IME、DPI、暗色主题和安装包冒烟尚未执行；这些属于发布验收，不把它们误记为已验证。
