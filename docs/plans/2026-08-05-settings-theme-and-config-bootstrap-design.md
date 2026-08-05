# Settings 主题可读性与配置目录初始化设计

> 日期: 2026-08-05
> 风险: low
> 关联: `docs/todo/todolist_02.md`、`docs/plans/2026-07-20-cross-platform-settings-design.md`

## 1. 目标

修复 Settings 窗口在系统深色模式下文字与按钮不可读的问题，并让用户能够从配置路径输入框旁的 `...` 按钮直接选择一个空目录，由 Settings 自动在其中生成默认配置。

对应 `todolist_02.md` 三条：

| # | 原始输入 | 归类 |
|---|---------|------|
| 1 | settings 里面文字都应该是黑色 | 主题可读性 |
| 3 | 配置项的删除和编辑都是白色了,以致于完全看不见按钮 | 主题可读性（同根因） |
| 2 | 配置文件夹输入框后面三个点，选择空目录后自动生成默认配置 | 配置目录初始化 |

## 2. 根因分析

### 2.1 第 1、3 条是同一个缺陷

`ui/settings.slint` 把所有容器色写死成浅色：窗口 `background: #f5f7fb`，卡片 `background: white`，标题 `color: #172033`。

但 `Button`、`LineEdit`、`TextEdit` 以及未显式设置 `color` 的 `Text` 取的是 std-widgets 调色板，而 `build.rs` 只调用了 `slint_build::compile()`，没有指定样式，因此使用默认的 `fluent`——**该样式会跟随操作系统的深色/浅色设置**。

Windows 处于深色模式时：

- `Palette.foreground` 变为近白色 → 未显式着色的 `Text`（未保存提示条、"New match / Edit match" 标题）在白卡片上不可见，即第 1 条。
- fluent-dark 的控件底色是叠加在父容器之上的**半透明白**，铺在白色卡片上几乎还是白色，而按钮文字是白色 → `Edit` / `Delete` 完全消失，即第 3 条。

所以这不是两个 bug，是"应用写死浅色 + 控件跟随系统深色"这一个不一致导致的两种表现。

### 2.2 三种可选修法

调研结论（Slint 官方文档 + slint-ui discussions，见 §7）：

| 方案 | 做法 | 优点 | 缺点 |
|------|------|------|------|
| A. 编译期钉死样式 | `build.rs` 用 `CompilerConfiguration::new().with_style("fluent-light")` | 零运行时代码；不存在启动瞬间闪深色；三端行为一致 | 系统深色模式下窗口仍是亮的 |
| B. 运行时强制 | `.slint` 里 `export { Palette }`，Rust 侧 `window.global::<Palette>().set_color_scheme(ColorScheme::Light)` | 可运行时切换 | 多一层导出与运行时代码；窗口创建到设值之间可能闪一帧深色 |
| C. 全量语义 token | 用 `Palette.background` / `.foreground` / `.control-background` 替换全部硬编码色，做到真正双主题 | 体验最好，深色用户友好 | 要改约 20 处颜色，且需要三端深浅两套人工验收 |

**选 A。** 理由：

1. 用户诉求明确是"文字都应该是黑色"，即要浅色确定性，而不是要深色适配。
2. A 是最小改动——一行 build 配置修掉两条缺陷，符合"每一行改动都能追溯到需求"的原则。
3. 现有 UI 的容器色本就全是浅色硬编码，钉死 fluent-light 后整窗自洽，不会留下半适配的中间态。
4. C 是更好的终局，但属于独立需求，本次不做，记入 §8 后续。

`ColorScheme` 语义容易记反，这里明确一次：`fluent-light` = 浅底深字，正是所需。除 Qt 样式外所有样式都支持强制指定色彩方案。

## 3. 配置目录初始化（第 2 条）

### 3.1 与现有迁移流程的关系

Settings 现在已有一条路径：`Choose new directory...` → `MigrationService::preflight` → `Copy and switch`，语义是**把现有配置复制到新空目录**。

第 2 条要的是另一种意图：**在空目录里从零生成一份默认配置**。两者共用"必须是空目录"的前置条件，但产物不同，因此建模为两个独立入口，而不是给迁移加分支：

```
路径输入框 ─┬─ [...]                → 选空目录 → 生成默认配置 → 切换
            ├─ [Open current directory]
            └─ [Choose new directory...] → 预检 → [Copy and switch]   （原有，不动）
```

`...` 成功后清空迁移面板的 `selected-path`，避免出现"刚生成完默认配置，又提示可以复制过去"的矛盾状态——目录此时已非空，迁移预检本来也会失败。

### 3.2 交互与错误预防

调研（NN/g 错误预防、Windows 选择器指南，见 §7）指向的取舍：

- **路径框保持只读，改动只经 `...`**。手打路径是高错误率输入，只读框 + 浏览按钮是桌面端标准做法，能从设计上消除一整类错误。
- **非空目录直接拒绝，不做"要不要合并"的二次确认**。合并进已有目录会产生无法预测的配置叠加，属于 Nielsen 所说"应当靠设计预防而不是靠报错补救"的场景。错误文案要说明**为什么**被拒绝以及**下一步怎么办**（新建一个空文件夹再选），不能只说"失败"。
- **选中即生成，不加确认步骤**。目录保证为空，写入不会覆盖任何东西，破坏性为零；此时插一个确认框只是无谓摩擦。这一点采用用户在 todolist 中的字面要求。
  - 备选方案（未采用）：选目录后展示"复制当前配置 / 生成默认配置"两个显式按钮，把两种意图收敛到一个选择器下。更统一，但多一次点击，且与第 2 条字面描述不符。若后续觉得两个入口容易混淆，这是首选的收敛方向。
- **明确告知需要重启**。espanso 的配置目录在进程启动时解析，切换后必须重启才生效，沿用迁移流程既有的提示措辞。

### 3.3 模板来源

默认模板 `default.yml` / `base.yml` 目前以 `include_str!` 内嵌在二进制 crate `espanso` 的 `src/res/config/` 下，由 `espanso::config::populate_default_config` 使用。

`espanso-settings` **不能**反向依赖二进制 crate（这是 2026-07-20 设计已确立的约束）。三种取法：

| 方案 | 评价 |
|------|------|
| `include_str!("../../espanso/src/res/config/default.yml")` | 跨 crate 穿透文件路径，破坏 crate 边界，一旦 espanso 目录结构调整就断 |
| 在 espanso-settings 里复制一份字面量 | 两处模板必然漂移 |
| **由调用方注入** | 采用 |

在 `SettingsLaunchOptions` 上新增 `templates: ConfigTemplates { default_yml, base_yml }`，由 `espanso/src/cli/settings.rs` 从既有的 `include_str!` 常量填入。单一事实来源仍在 espanso crate，settings crate 保持纯粹且可单测（测试里注入最小模板即可）。

### 3.4 写入策略

复用迁移已经验证过的"暂存 → 校验 → 原子发布"三段式，不发明新流程：

```
preflight(dest)          绝对路径 / 是目录或可创建 / 目录为空 / 与当前配置目录不重叠
  → staging = dest.espanso-scaffold-<pid>-<nonce>
  → 写 staging/config/default.yml、staging/match/base.yml
  → EspansoConfigValidator::validate(staging)      espanso_config::load 全量校验
  → 删除空的 dest，rename(staging, dest)           原子发布
  → ConfigLocationStore::save_atomic(dest)         持久化选择
```

任何一步失败都在 staging 目录内失败，活动配置目录不受影响——与迁移失败保护的语义一致。

目录布局与 `populate_default_config` 保持一致（`config/default.yml` + `match/base.yml`），否则从 Settings 生成的目录和从首次启动生成的目录会不一样。

## 4. 影响面

- `espanso-settings/build.rs`：改用 `compile_with_config` 钉死 `fluent-light`。
- `espanso-settings/src/scaffold.rs`（新增）：`ScaffoldService::preflight` / `execute`。
- `espanso-settings/src/lib.rs`：导出 `ConfigTemplates`、`ScaffoldService`；`SettingsLaunchOptions` 增加 `templates` 字段。
- `espanso-settings/ui/settings.slint`：路径框旁增加 `...` 按钮与 `browse-config()` 回调。
- `espanso-settings/src/app.rs`：绑定 `browse-config`。
- `espanso/src/cli/settings.rs`：注入模板常量。
- 不改数据库、网络协议、匹配引擎、YAML 格式。

## 5. 验收标准

- [ ] 系统处于深色模式时，Settings 窗口全部文字为深色，`Edit` / `Delete` 按钮可见。
- [ ] `...` 弹出系统目录选择框；选空目录后目录内出现 `config/default.yml` 与 `match/base.yml`，路径框更新，并提示重启。
- [ ] 选择非空目录被拒绝，错误文案说明原因与下一步，且不写入任何文件。
- [ ] 生成失败时活动配置目录不变，不留下 staging 残留目录。
- [ ] `cargo fmt --check`、espanso-settings 严格 Clippy、crate 测试通过。

## 6. 回滚方案

- 主题：`build.rs` 改回 `slint_build::compile()` 即恢复跟随系统。
- 初始化：移除 `...` 按钮即关闭入口；已生成的目录是普通配置目录，删掉 `settings-location.json` 即可回到原路径解析。

## 7. 调研依据

深度调研（三轮，SearXNG 与私有 firecrawl 本机不可用，降级至 WebSearch + 官方文档抓取）。

| # | 结论 | 可信度 | 来源 |
|---|------|--------|------|
| 1 | `fluent` / `material` / `cupertino` / `cosmic` 均有 `-light` / `-dark` 变体，选定变体即覆盖系统设置 | ⭐⭐⭐ | [Widget Styles, Slint Docs](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/) |
| 2 | Rust 侧编译期指定样式用 `slint_build::compile_with_config` + `CompilerConfiguration::new().with_style(...)`；亦可用 `SLINT_STYLE` 环境变量 | ⭐⭐⭐ | [`compile_with_config`, docs.rs](https://docs.rs/slint-build/1.17.1/slint_build/fn.compile_with_config.html)、[Widget Styles](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/) |
| 3 | `Palette.color-scheme` 是 in-out 属性，可读可写以强制深浅；Qt 样式除外 | ⭐⭐⭐ | [Builtin Global Singletons](https://releases.slint.dev/1.7.0/docs/slint/src/language/builtins/globals)、[Widget Styles](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/) |
| 4 | 从 Rust 改 `color-scheme` 必须先在自己的 `.slint` 里 `export { Palette }`，再 `global::<Palette>().set_color_scheme(...)`；1.15.1 起可用（本项目 1.17.1 满足） | ⭐⭐ | [slint-ui discussion #9550](https://github.com/slint-ui/slint/discussions/9550) |
| 5 | 错误应当靠设计预防而非事后补救；error 是阻断性的，warning 是可继续的，非空目录属于前者 | ⭐⭐⭐ | [NN/g: Hostile Error Messages](https://www.nngroup.com/articles/hostile-error-messages/)、[Nielsen 十大可用性原则](https://shiftasia.com/community/applying-jakob-nielsens-10-usability-heuristics-for-better-ux-design/) |
| 6 | 桌面端标准是选择器返回路径后以只读形式回显，不鼓励手工输入路径 | ⭐⭐ | [Windows App SDK 文件/文件夹选择器](https://learn.microsoft.com/en-us/windows/apps/develop/files/using-file-folder-pickers) |
| 7 | Obsidian 建库明确区分"新建空库"与"使用已有文件夹"两种意图，且新建后不自动铺默认文件 | ⭐⭐ | [Obsidian: Create a vault](https://obsidian.md/help/vault) |
| 8 | espanso 配置目录在进程启动时解析，改动后需 `espanso restart` | ⭐⭐⭐ | [Espanso Configuration Basics](https://espanso.org/docs/configuration/basics/)、[issue #2382](https://github.com/espanso/espanso/issues/2382) |

结论 7 与本次取舍相反（Obsidian 不铺默认文件），保留记录：espanso 与 Obsidian 的差别在于空配置目录对 espanso 无意义——没有 `default.yml` 就没有可用配置，所以这里自动生成是合理的偏离。

## 8. 后续（本次不做）

- 语义 token 层（§2.2 方案 C）：把硬编码色收敛到一处，之后支持跟随系统深色只需放开一个开关。
- `...` 与 `Choose new directory...` 两个入口若在实际使用中造成混淆，按 §3.2 备选方案收敛为单选择器 + 两个显式动作。
- 三端（Windows / macOS / Linux）真实窗口、DPI、IME 人工冒烟仍未执行，沿用 2026-07-20 设计中的待办。
