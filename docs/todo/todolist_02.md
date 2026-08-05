## 原始输入（原文，勿改）
- 1. settings 里面文字都应该是黑色
- 2. 配置文件夹 输入框后面三个点,点击后可以弹出系统选择文件夹的选择框,选择(必须是空)完成后,自动生成默认配置文件夹的一些配置到该文件夹
- 3. 配置项的删除和编辑都是白色了,以致于完全看不见按钮

> 状态: ✅ 已完成 (3/3)
> 分支: `dev`
> 更新: 2026-08-05
> 设计: `docs/plans/2026-08-05-settings-theme-and-config-bootstrap-design.md`

- ✅ 1. settings 里面文字都应该是黑色
- ✅ 2. 配置文件夹 输入框后面三个点,点击后可以弹出系统选择文件夹的选择框,选择(必须是空)完成后,自动生成默认配置文件夹的一些配置到该文件夹
- ✅ 3. 配置项的删除和编辑都是白色了,以致于完全看不见按钮

## 说明与上下文（完成后补）

- 做了什么：
  - 第 1、3 条是同一个根因。窗口自己画浅色底（`#f5f7fb` + 白卡片），但 std-widgets 跟随系统色彩方案；系统开深色时 `Palette.foreground` 变近白色，且 fluent-dark 控件底色是叠在父容器上的半透明白，铺在白卡片上仍是白 → 文字变浅、`Edit`/`Delete` 完全消失。`build.rs` 改用 `compile_with_config` + `with_style("fluent-light")` 在编译期钉死浅色，一处修掉两条。
  - 第 2 条新增 `...` 浏览按钮与 `ScaffoldService`：选目录 → 校验为空且不与当前配置目录重叠 → 在 staging 目录写 `config/default.yml` + `match/base.yml` → 用 `espanso_config::load` 全量校验 → rename 原子发布 → 持久化位置 → 提示重启。
- 关键决策：
  - 钉死浅色而非做双主题。用户诉求是"文字都应该是黑色"，且现有容器色本就全是浅色硬编码；语义 token 层（真正支持深色）记为后续，见设计文档 §8。
  - 默认模板由调用方注入（`SettingsLaunchOptions.templates`），而不是跨 crate `include_str!` 或复制字面量——`espanso-settings` 不能反向依赖二进制 crate `espanso`，单一事实来源仍在 `espanso/src/res/config/`。
  - 非空目录直接拒绝、不提供合并选项（错误预防优于事后报错）；目录保证为空所以选中即生成，不加确认步骤。
  - `...` 与原有 `Choose new directory...` + `Copy and switch` 是两个独立意图（从零生成 / 复制现有），生成成功后清空迁移面板避免矛盾状态。
- 涉及文件：`espanso-settings/{build.rs, ui/settings.slint, src/{lib.rs, app.rs, scaffold.rs, migration.rs}, tests/{api.rs, scaffold.rs}}`、`espanso/src/{config.rs, cli/settings.rs}`。
- 验证结果：`cargo fmt --all --check` 通过；`cargo clippy -p espanso-settings --all-targets` 零警告（含 UI feature，即 `fluent-light` 样式编译通过）；`cargo test -p espanso-settings` 通过。
- 风险与后续：三端（Windows/macOS/Linux）真实窗口下的深色模式观感、`...` 选择器行为、DPI 与 IME 仍需人工冒烟，未执行。`espanso-ipc/src/windows.rs` 存在 4 处既有的 `uninlined_format_args` clippy 告警（本次未触碰，Windows 专属代码，CI 在 Linux 上不编译该文件）。
