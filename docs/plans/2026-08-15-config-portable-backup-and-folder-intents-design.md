# 配置目录意图分流与可移植备份/迁移设计

> 日期: 2026-08-15
> 风险: medium（触及 Settings 目录切换与磁盘读写；导出/导入会新增写路径，但默认不覆盖活动配置）
> 关联: `docs/todo/todolist_04.md`、`docs/plans/2026-08-12-adopt-existing-config-and-echo-matches-design.md`、`docs/plans/2026-08-05-settings-theme-and-config-bootstrap-design.md`、`docs/plans/2026-07-20-cross-platform-settings-design.md`

## 1. 目标

用户反馈（原文意图，合并表述）：

| # | 用户诉求 | 归类 |
|---|---------|------|
| 1 | 选择配置文件夹时仍然提示必须为空；要求**可以不为空** | 目录意图分流仍不完整 / 入口易走错 |
| 2 | 或者能**导出再导入**，方便迁移和备份 | 缺少可移植备份单元（archive） |

本次交付是**架构设计**，不直接改代码。设计要同时解决：

1. 为什么在 2026-08-12 已做「接纳非空 espanso 目录」后，用户仍会撞上「必须为空」。
2. 如何用符合桌面最佳实践的信息架构，把「打开已有 / 新建 / 搬家复制 / 备份导出 / 从备份恢复」拆成不可混淆的意图。
3. 导出/导入应长什么样：格式、校验、覆盖策略、与现有 `Adopt` / `Scaffold` / `Migration` 的边界。

**非目标（本次设计明确不做实现细节拍板之外的扩展）：**

- 云同步账户体系（Dropbox/GitHub OAuth）。
- 跨机实时协作 / 冲突合并 UI。
- 把外部手写 YAML 变成可写（仍遵守 2026-08-12 的只读边界）。
- 替换 Espanso 官方推荐的 symlink 同步方案（见 [Sync your configuration](https://espanso.org/docs/sync/)）。

## 2. 现状与根因

### 2.1 代码里其实有三条路径，文案却像两条

Settings 页「Configuration directory」当前入口：

| UI 控件 | 回调 | 服务 | 对非空目录的态度 |
|---|---|---|---|
| 路径旁 `...` | `browse-config` | `AdoptService::plan` | 空 → 生成默认；已有 espanso（含 `config/` 且 load 通过）→ **接纳零写入**；其它非空 → 拒绝 |
| `Choose new directory...` | `choose-folder` + `migrate` | `MigrationService` | **目标必须为空**（`destination must be empty`） |
| （无） | — | — | **没有** Export / Import 备份包 |

相关实现：

- 接纳分流：`espanso-settings/src/adopt.rs`
- 脚手架仍强制空：`espanso-settings/src/scaffold.rs`（`the selected directory is not empty; create a new empty folder and select it`）——只在「空目录生成」分支使用，正确。
- 搬家复制强制空：`espanso-settings/src/migration.rs:65`（`destination must be empty`）——**这是用户最容易再撞上的那句**。
- UI 文案：`settings.slint` 上半段仍写 “Choosing a new directory runs a preflight check first, then **copies**…”，下半段又写 `...` 可采纳已有配置。两个按钮语义相邻、动词都是 “choose/directory”，用户无法凭按钮文本预判哪条路径允许非空。

### 2.2 用户仍看到「必须为空」的三种真实路径

按出现概率排序：

1. **点了 `Choose new directory...` 而不是 `...`**  
   迁移路径语义是「把当前配置**复制**到新位置并切换」。复制目标若已有文件，直接覆盖会静默毁数据，因此强制空目录在**复制语义**下是对的。问题是：用户想要的往往是「打开我已有的那份」，却被引导到了复制入口。

2. **点了 `...`，但选中的不是 espanso 配置根**  
   例如选了 `match/` 子目录、只含零散 yml 的备份夹、或解压后多了一层 `espanso/` 父目录。`AdoptService` 要求 `selected/config` 为目录，否则报：
   > not empty and does not contain a 'config' folder…

   这句**不是** “must be empty”，但体感同样是「非空就不让用」。若用户把整句读成拒绝非空，会与 2026-08-12 的承诺冲突。

3. **运行的仍是旧二进制 / 未合入 #6 的构建**  
   2026-08-12 的接纳逻辑在 PR #6。若本机 Settings 仍是旧构建，`...` 会继续走旧的「仅空目录」脚手架。设计文档需在验收里要求「用含 adopt 的构建验证」，避免把实现问题误判成产品缺口。

### 2.3 缺的不是「再放宽一次空检查」，而是「备份单元」

Espanso 配置本质是**目录形态的数据**（`config/` + `match/` + 可选 packages）。官方迁移/备份做法是：停服务 → 拷贝整个配置目录 → 或 symlink 到云盘（[Sync docs](https://espanso.org/docs/sync/)）。社区与同类产品（KeePass 的 `.kdbx`、VS Code 的 `.code-profile`、TextExpander/PhraseExpress 的 group 导出）都表明：

> 用户要的「方便迁移和备份」= **一个可拿走的文件 + 明确的恢复动作**，而不是再学会一次「选对空文件夹再 Copy and switch」。

当前 Settings 只有「指过去」(adopt) 和「搬过去」(migrate-copy)，没有「打包带走 / 打包还原」。所以即使用户已经能 adopt，**跨机器、发同事、定期备份**仍要自己 zip，且没有校验与一键恢复。

## 3. 调研结论（deep，中英多源）

SearXNG-public 本机 token/helper 不可用，降级 WebSearch + 官方文档抓取。与 2026-08-12 的结论衔接，本轮新增「备份/导入」维度。

| # | 结论 | 可信度 | 对方案的约束 | 来源 |
|---|------|--------|--------------|------|
| 1 | **Open existing 与 Create new 必须是并列、等权入口**；Open 不得要求空目录 | ⭐⭐⭐ | UI 上拆开意图，而不是一个 `...` 硬猜全部 | [Obsidian Manage vaults](https://obsidian.md/help/manage-vaults)、[Cryptomator Adding vaults](https://docs.cryptomator.org/desktop/adding-vaults/) |
| 2 | 文件夹型应用的「打开」= **指过去、零写入**（或仅补自己的元数据） | ⭐⭐⭐ | 继续保留 `Adopt` 零写入契约 | Obsidian “Open folder as vault”；VS Code Open Folder |
| 3 | **备份格式应是原生数据的无损打包**，不要用「会丢字段的中间格式」当主备份 | ⭐⭐⭐ | 主备份 = 整棵配置树的 archive；CSV 只可作次要导出 | [KeePass backup guidance](https://www.panicvault.org/keepass/backup-database/)、KeePass「别拿明文 CSV 当备份」 |
| 4 | 文本扩展器行业标配 **Export group / Import group**（原生或 CSV 桥） | ⭐⭐⭐ | Settings 应提供一等公民的 Export/Import | [TextExpander import/export](https://textexpander.com/learn/using/importing-and-exporting-snippet-groups)、[PhraseExpress export](https://www.phraseexpress.com/doc/phrase-files/export/) |
| 5 | 导入 UX：短说明 + 明显主按钮 + **校验 → 预览 → 确认**；覆盖前提醒 | ⭐⭐⭐ | Import 必须 preflight + 摘要 + 显式确认 | [Smashing Magazine: data importer UX](https://www.smashingmagazine.com/2020/12/designing-attractive-usable-data-importer-app/) |
| 6 | VS Code 同时提供 **持续同步** 与 **文件型 profile 导出**；二者解决不同问题 | ⭐⭐⭐ | 不把 symlink/云同步塞进 Settings 第一屏；文件导出解决「带走一份」 | [VS Code Profiles](https://code.visualstudio.com/docs/configure/profiles)、[Settings Sync](https://code.visualstudio.com/docs/configure/settings-sync) |
| 7 | Espanso 官方同步/迁移以**整目录 + symlink/env** 为主，无官方 `.espanso-backup` | ⭐⭐⭐ | 我们定义的 archive 是 Settings 层能力，不与引擎抢职责；内容应对齐 `espanso path config` 树 | [espanso.org/docs/sync](https://espanso.org/docs/sync/)、[Configuration basics](https://espanso.org/docs/configuration/basics/) |
| 8 | 入口命名必须描述真实行为，否则等于坏功能 | ⭐⭐ | 禁再用含糊的 “Choose new directory” 同时承担 adopt/migrate | KeePassDX 命名争议、Cryptomator “Add existing” 讨论 |
| 9 | 复制/迁移目标为空是**保护性约束**，不是产品缺陷；缺陷是用户被错误引导到复制 | ⭐⭐⭐ | 保留 migrate 的空目标约束；用信息架构把用户引到 adopt/import | 现有 `MigrationService` 安全模型 |
| 10 | 错误文案要说明原因 + 下一步（NN/g） | ⭐⭐⭐ | 非 espanso 非空目录应提示「选含 config/ 与 match/ 的根」或「先 Import 备份包」 | 延续 2026-08-12 §8.9 |

**综合判断：**

- 「允许非空」在 **Open/Adopt** 语义下已经是正确目标；要修的是**入口可发现性**与**选错根目录时的引导**，不是删掉 migrate 的空检查。
- 「导出再导入」是独立能力，对标 VS Code profile / KeePass 文件 / 文本扩展器 group export，应成为迁移与备份的**主路径**；「Copy and switch」降为「本机换盘符/换路径」的次要路径。

## 4. 信息架构：四种意图，四个动作

用户对配置目录只有四类真实意图。每类对应一个动作、一套前置条件、一种磁盘效应：

```
                    ┌─────────────────────────────────────────────┐
                    │         Configuration directory card        │
                    │  Current: <path>   [Open in file manager]   │
                    └─────────────────────────────────────────────┘
                      │            │              │            │
          ┌───────────▼──┐  ┌──────▼──────┐  ┌───▼────────┐  ┌─▼──────────┐
          │ Open existing│  │ Start fresh │  │ Export     │  │ Import     │
          │ configuration│  │ (empty dir) │  │ backup…    │  │ backup…    │
          └──────┬───────┘  └──────┬──────┘  └───┬────────┘  └─┬──────────┘
                 │                 │              │             │
                 ▼                 ▼              ▼             ▼
              Adopt            Scaffold        Archive       Restore
           (0 writes)      (write defaults)   (read-only    (write to
                                               zip out)     chosen target)
```

| 意图 | 按钮文案（建议） | 服务 | 目标可否非空 | 写磁盘？ |
|------|------------------|------|--------------|----------|
| 打开已有配置 | **Open existing configuration…** | `AdoptService` | **必须非空且像 espanso**（或可探测的备份解压根） | 否（只改 location store） |
| 在空目录新建 | **Create new configuration…** | `ScaffoldService` | **必须空/不存在** | 是（默认文件） |
| 备份带走 | **Export backup…** | 新增 `BackupService::export` | N/A（选文件保存位置） | 是（只写用户选的 zip） |
| 从备份恢复 | **Import backup…** | 新增 `BackupService::import` | 见 §5.3 | 是（写入目标目录，确认后） |
| 本机复制搬家（次要） | **Copy to another folder…** | 现有 `MigrationService` | 目标必须空 | 是（复制树） |

### 4.1 入口收敛规则

1. **删除或降级** 当前含糊的 `Choose new directory...` 主按钮文案。  
   - 主区只保留：Open existing / Create new / Export / Import。  
   - `Copy to another folder…` 放在次级（「Advanced」折叠或底部链接），避免再抢主路径。

2. **`...` 不再身兼多职。**  
   - 方案 A（推荐）：路径旁 `...` = **Open existing** 的快捷方式（只 adopt，不再 scaffold）。  
   - Create new 走单独按钮 → 只 scaffold。  
   - 这样「点 `...` 却撞上 empty 检查」在 Create 路径才可能发生，且文案已自证。

3. **自动分流可以保留在 Open 内部，但只用于「纠错」**，不再用于「猜用户要新建还是打开」：  
   - Open existing 选到空目录 → 错误：`This folder is empty. Use “Create new configuration…” instead.`  
   - Create new 选到非空 espanso → 错误：`This folder already looks like an Espanso configuration. Use “Open existing…”.`  
   - Create new 选到非空非配置 → 保持现有 empty 拒绝。  
   - 这是把 2026-08-12 的「一个按钮自动分流」升级为「按钮表达意图 + 分流失手时互相指路」。Cryptomator 的显式入口与 Obsidian 的并列入口在此会合。

### 4.2 与 2026-08-12 的关系

| 2026-08-12 | 本次 | 关系 |
|------------|------|------|
| `AdoptService` 零写入接纳 | 保留，成为 **Open existing** 唯一后端 | 不重写，只改绑定入口 |
| 非空非配置拒绝 | 保留，但文案增加「或 Import backup / 选对根目录」 | 体验补全 |
| 外部 match 只读回显 | 不动 | 导入后的目录同样走 inventory 扫描 |
| `MigrationService` 复制 | 降为 Advanced | 空目标约束**保留** |

## 5. 可移植备份（Export / Import）架构

### 5.1 备份单元：配置树 archive，不是「设置 JSON」

Espanso 的用户资产是**整棵配置目录**（含多文件 match、注释、packages 引用结构）。因此：

- **主格式**：`.espanso-backup.zip`（或 `.zip` 内含清单；扩展名用于双击/过滤器识别）。
- **内容**：以当前活动配置根为源，打包 `config/**`、`match/**`；若 packages 目录在配置根约定位置内则一并打包；**不打包** runtime / 日志 / 缓存。
- **清单文件**（archive 根目录 `espanso-backup.json`）：

```json
{
  "format": "espanso-backup",
  "format_version": 1,
  "espanso_min_version": "2.3.0",
  "exported_at": "2026-08-15T13:00:00+08:00",
  "source_path_hint": "C:\\Users\\…\\espanso",
  "file_count": 42,
  "byte_count": 128000,
  "includes": ["config", "match"]
}
```

设计原则对齐 KeePass：「备份 = 原生数据的加密/打包拷贝」；明文中间格式只用于迁移到**别的产品**，不作为本产品主备份。

### 5.2 Export 流程

```
Export backup…
  → 读取 active config root（只读 walk，禁 follow symlink，复用 migration 的 symlink 冲突策略）
  → 在内存/临时目录写 espanso-backup.json
  → 打 zip 到用户选择的保存路径（rfd save file）
  → status: "Backup exported: N files (M bytes)."
  → 失败不碰活动配置
```

约束：

- Export **永不**修改活动配置。
- 默认文件名：`espanso-backup-YYYYMMDD-HHMM.zip`。
- 大目录显示进度文案即可（与现有 migrate 后台线程模式一致）；首版可不做取消。

### 5.3 Import 流程（校验 → 预览 → 确认）

```
Import backup…
  → 选 .zip / .espanso-backup.zip
  → 解压到临时 staging
  → 读清单；无清单则尝试把 staging 当「裸配置根」探测（兼容用户手 zip 的目录）
  → espanso_config::load(staging) 校验（error 阻断；warning 进预览）
  → 弹出/内嵌预览摘要：
        将导入 N 个 config、M 个 match
        警告文件列表（可折叠）
        目标位置： [ 当前配置目录 | 新选目录… ]
        若目标非空：必须二选一
           (a) 取消
           (b) 写入新空目录后切换（推荐默认）
           (c) 覆盖当前目录 —— 仅当用户勾选危险确认
  → 确认后：
        推荐路径：staging 校验通过 → 发布到目标（空目录 rename 或「新目录」）
                 → ConfigLocationStore 切换 → reload_matches
        覆盖路径：先把当前目录导出自动 safety-backup 到邻接 `*.pre-import-YYYYMMDD.zip`
                 → 再替换 → 切换
  → 提示重启 Espanso
```

**为什么默认不覆盖当前目录：**  
Smashing Magazine / 通用 importer UX 要求覆盖前有预览与确认；KeePass 社区强调 restore 先到临时位置验证。默认「导入到新目录再 adopt」把破坏半径降到零，与 `Adopt` 的安全叙事一致。

**手 zip 兼容：**  
用户常把 `%APPDATA%\espanso` 直接打成 zip。Import 若找不到清单，应探测：

1. staging 根即含 `config/`；或  
2. staging 下唯一子目录含 `config/`（去掉多余包装层）。

探测失败再报错，并提示正确结构。

### 5.4 Import 与 Adopt / Migration 的边界

| 能力 | 输入 | 输出 | 是否复制树 |
|------|------|------|------------|
| Open existing (Adopt) | 磁盘上已有配置目录 | 只改 location 指针 | 否 |
| Create new (Scaffold) | 空目录 | 写入默认 yml | 是（生成） |
| Copy to folder (Migration) | 当前目录 → 空目标 | 复制后切换 | 是 |
| Export backup | 当前目录 → zip 文件 | 备份文件 | 只读源 |
| Import backup | zip → 目标目录 | 恢复树并可选切换 | 是（受控） |

Import **不是** Adopt：Adopt 假定目录已在最终位置；Import 负责「把便携包落到某个位置」。落盘后应复用 `AdoptService::inspect` + `reload_matches`，不要第二套加载逻辑。

### 5.5 模块落点（建议）

```
espanso-settings/src/
  backup.rs          // BackupService { export, inspect_archive, import }
  adopt.rs           // 不变契约；Open existing 绑定
  scaffold.rs        // 仅 Create new
  migration.rs       // Advanced: Copy to another folder
  app.rs             // 绑定四个主回调 + advanced
  ui/settings.slint  // 信息架构重排
tests/backup.rs      // 导出往返、坏 zip、包装层探测、拒绝覆盖未确认
```

`BackupService` 依赖：

- `walkdir` + 现有 `copy_tree` / symlink 拒绝策略（可抽 `tree.rs` 以免 migration/backup 分叉）。
- 压缩：优先 `zip` crate（已在 Rust 生态常用）；若仓库已有压缩依赖则复用，避免双栈。
- 校验：`EspansoConfigValidator` 的「严格」用于**我们刚写出的**目标；Import 预览阶段用 `AdoptService::inspect` 的「宽松」（warning 可继续），与 2026-08-12 §4.2 一致。

## 6. UI 线框（Settings 页 Configuration directory 卡片）

```
Configuration directory
Current: D:\…\espanso                    [Open folder]

Primary actions
  [ Open existing configuration… ]   [ Create new configuration… ]
  [ Export backup… ]                 [ Import backup… ]

Help (单行，可 wrap)
  Open points at a folder that already has config/ and match/.
  Create needs an empty folder. Export/Import use a portable backup file
  for migration between machines.

Advanced
  ▸ Copy current configuration to another empty folder…
      (现有 selected-path + summary + Copy and switch)
```

错误区沿用现有 `error-message` 红条；成功走 `status-message`。Import 预览可用现有卡片内嵌块（与 migration summary 同模式），避免首版上系统对话框框架。

### 6.1 关键文案（英文 UI，与仓库现状一致）

| 场景 | 文案要点 |
|------|----------|
| Open 选到空目录 | empty → use Create new |
| Open 选到非配置非空 | missing `config/`; pick the folder that contains `config` and `match`, or Import a backup |
| Create 选到已有配置 | already an Espanso configuration → use Open existing |
| Create 选到其它非空 | must be empty（保留） |
| Copy 目标非空 | destination must be empty（保留）+ 提示改用 Open/Import |
| Import 校验失败 | archive is not a usable Espanso configuration; nothing was written |
| Import 成功 | Restored N config(s), M match(es) to \<path\>. Restart Espanso. |

## 7. 状态机（目录切换相关）

```
                    open_existing
                 ┌───────────────┐
                 ▼               │
[ActiveConfig] ──adopt_ok──► [ActiveConfig'] ──export──► (file)
      │                ▲
      │ create_new     │ import_ok (switch)
      ▼                │
  scaffold_ok ─────────┘
      │
      │ copy_to (advanced)
      ▼
  migration_ok ──► [ActiveConfig']
```

所有成功切换后统一：

1. `ConfigLocationStore::save_atomic`
2. 替换 `UiMatchRepository`
3. `reload_matches`（含 external inventory）
4. 提示重启 Espanso  

Import 若选择「只恢复不切换」，则跳过 1–3，只写目标目录并提示路径——首版可**不做**该分支，强制恢复后切换，减少半成品状态。

## 8. 威胁与安全

| 风险 | 缓解 |
|------|------|
| Zip slip（`../` 写穿） | 解压时规范化路径，拒绝跳出 staging 根 |
| 符号链接逃逸 | Export/Import 均 `follow_links(false)`；遇 symlink 记入 conflicts 并失败（与 migration 一致） |
| 覆盖活动配置 | 默认禁止；危险路径强制 safety-backup + 二次确认文案含路径 |
| 恶意 yaml | 仅用 `espanso_config::load` 解析，不反序列化任意类型；与引擎同信任级 |
| 备份含敏感替换文本 | 文档提示备份文件等同配置明文；不放公共云未加密处（Settings 内一行隐私提示） |

## 9. 验收标准（实现阶段用）

### 9.1 意图分流

- [ ] Open existing：有效 espanso 根 → 切换成功、零写入目标目录、Configuration 页回显。
- [ ] Open existing：空目录 → 明确指向 Create new，不生成文件。
- [ ] Create new：空目录 → 生成默认配置并切换。
- [ ] Create new：非空 → 拒绝；若可识别为 espanso 则指向 Open existing。
- [ ] Copy advanced：目标非空仍拒绝（`destination must be empty`），文案提示 Open/Import。
- [ ] 主按钮不再出现含糊的 “Choose new directory…”。

### 9.2 Export / Import

- [ ] Export 生成可打开的 zip，含 `espanso-backup.json` 与完整 `config/`、`match/`。
- [ ] Export 失败不影响活动配置。
- [ ] Import 合法包 → 预览摘要 → 确认 → 目标得到等价树 → load 通过 → 切换并回显 match。
- [ ] Import 坏包 / zip slip / 缺 config → 失败且目标无脏文件（staging 清理）。
- [ ] 用户手 zip 的「多一层 espanso/ 包装」可被探测导入。
- [ ] 覆盖当前目录路径（若实现）：先有 safety-backup，再替换。

### 9.3 回归

- [ ] 2026-08-12 外部 match 只读、Save deletion 不 panic 等行为保持。
- [ ] `cargo test -p espanso-settings`、fmt、clippy 全绿。
- [ ] 用**含本设计实现**的构建做人工冒烟（避免旧二进制假阴性）。

## 10. 分阶段交付

| 阶段 | 内容 | 价值 |
|------|------|------|
| **P0** | 信息架构重排：Open / Create 分钮；`...` 只 Open；Advanced 收纳 Copy；错误文案互指 | 立刻消掉「想打开却撞 must be empty」 |
| **P1** | `BackupService::export` + UI Export | 用户能带走一份标准备份 |
| **P2** | `BackupService::import`（默认导入到新目录并切换）+ 手 zip 探测 | 闭环迁移/恢复 |
| **P3** | 可选：覆盖当前目录 + 自动 safety-backup；Export 进度；把 backup 挂到 CLI | 进阶 |

**建议实现顺序严格按 P0 → P1 → P2。**  
若只做「放宽 migrate 空检查」而不做 P0/P1，会在错误语义上制造覆盖风险，且仍解决不了跨机备份。

## 11. 回滚

- P0：UI 文案与回调映射可逆；不改磁盘格式。
- P1/P2：删除 `backup.rs` 与按钮即可；已导出的 zip 仍是普通 zip，无强制升级负担。
- location store 格式不变。

## 12. 开放问题（实现前需拍板）

1. **扩展名**：`.espanso-backup.zip`（清晰）vs 普通 `.zip`（更好分享）？  
   **建议**：过滤器两者都收；默认保存 `.espanso-backup.zip`。
2. **packages 目录**：若本机 packages 在配置根外（`espanso path packages`），Export 是否跟随？  
   **建议**：P1 只打包配置根内可见树；根外 packages 在预览里 warning「not included」。
3. **Import 是否允许合并进当前 match（而非整树替换）？**  
   **建议**：不做。合并会碰到 trigger 冲突与只读外部文件策略；整树恢复更可预测。
4. **是否暴露 CLI**（`espanso backup export|import`）？  
   **建议**：P3；先 UI 闭环。

## 13. 结论

用户说的「还是必须为空」与「导出再导入」不是二选一，而是同一信息架构缺口的两面：

1. **打开已有** 与 **复制搬家** 现在挤在相似文案下，复制路径合法地要求空目录，于是「非空」诉求被错误路径挡住。  
2. **备份/跨机迁移** 需要文件型可移植单元；目录 adopt 解决不了「带走一份」。

正确产品形状是：

> **Open existing（非空、零写入）· Create new（空目录）· Export backup · Import backup**  
> Advanced 才是 Copy to empty folder.

实现按 P0→P2 推进即可同时兑现「可以为非空」与「能导出再导入」，并与 Obsidian / VS Code / KeePass / 文本扩展器行业惯例对齐，同时不破坏 2026-08-12 已建立的外部 YAML 只读安全边界。
