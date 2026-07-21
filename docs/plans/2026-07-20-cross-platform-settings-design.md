# Espanso 跨平台 Settings 与配置编辑器设计

> 日期: 2026-07-20
> 风险: medium
> 关联: `plan/2026-07-20-cross-platform-settings-config-editor-plan.md`

## 1. 目标

让 Windows、macOS、Linux 用户能从托盘打开同一套 Settings 界面，安全切换配置目录，并在不手写 YAML 的情况下管理基础文本替换规则。

## 2. 输入

- 启动输入：现有 config、packages、runtime 路径及其覆盖来源。
- 设置输入：用户选择的绝对目录和“复制并切换”确认。
- 规则输入：单行 trigger、至少支持 100,000 个 Unicode 字符的多行 replacement。
- 触发入口：托盘 `Settings…` 或隐藏命令 `espanso settings`。

## 3. 输出与副作用

- 显示 Slint 双选项卡窗口：`设置`、`配置`。
- 将持久化目录写入默认配置目录的同级文件 `espanso-settings-location.json`。
- 迁移时复制并验证配置，成功后切换；旧目录始终保留。
- 仅写 `<config>/match/ui.yml`，替换前生成 `ui.yml.backup`。
- 通过现有 worker IPC 打开或聚焦单实例 Settings。

## 4. 边界

- 不修改用户其他 YAML，不提供高级变量、表单、脚本规则的图形编辑。
- 不自动删除旧配置目录，不静默合并非空目标目录。
- 不引入 WebView、Node.js 或平台专属 UI 分支。
- 本次不发布、不部署；三端人工 UI/IME 验证需要对应平台环境。

## 5. 影响面

- 新增 `espanso-settings` crate：UI、位置存储、迁移、规则仓储、窗口单实例。
- 主程序：路径解析、隐藏 CLI、Settings 启动。
- worker/engine：托盘菜单事件与 IPC 转发。
- CI：新增 Settings crate 的三端 check/test。
- 不改数据库、网络协议、文本展开核心匹配算法。

## 6. 复用与兼容策略

- 复用 `espanso_config::load` 做完整配置验证，复用现有 `espanso-ipc` 和托盘菜单事件链。
- 用新增枚举变体和隐藏子命令实现加法兼容；现有命令与 YAML 保持不变。
- 路径优先级固定为 CLI > 环境变量 > 持久化选择 > 现有平台路径解析。
- 主程序把私有 `path::Paths` 映射为 `espanso_settings::SettingsPaths`，避免反向依赖二进制 crate。

## 7. 验收标准

- [x] 位置优先级、原子存储、迁移失败保护、YAML 往返与校验单测通过。
- [x] `cargo fmt --check`、目标 crate 严格 Clippy、相关测试通过。
- [ ] Settings crate 在 Windows、macOS、Linux CI 编译。
- [x] 托盘、CLI、IPC 与单实例聚焦链路已实现并通过编译/单测；真实窗口行为待三端人工冒烟。
- [x] 路径迁移失败不改变活动目录，规则保存失败不破坏最后有效文件。
- [x] diff 不包含凭据、调试残留或无关格式化。

## 8. 回滚方案

- 移除 `Settings…` 菜单和隐藏子命令即可停止入口，不影响展开引擎。
- 删除 `settings-location.json` 后恢复现有路径解析。
- 原目录不会被删除；规则异常时可恢复 `match/ui.yml.backup`。

## 9. Watch 计划

- 本次只交付代码，不执行部署，因此没有线上 Watch 窗口。
- 合入前三端 smoke；未来打包发布后按 medium 风险观察启动失败和配置加载错误 30 分钟。
