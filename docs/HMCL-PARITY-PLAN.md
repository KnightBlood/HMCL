# HMCL Java → C# / Rust 功能与 UI 对齐任务清单

> 依据：本仓库 `HMCL/`（Java 应用，`org.jackhuang.hmcl`）与 `HMCLCore/`（Java 核心库）源码。
> 目标：C#（`src/HMCLCore.NET` + `src/HMCL.NET`）与 Rust（`hmcl-rs/`）在**功能**与 **UI 布局/交互**上对标 Java 版。
> 状态标记：`[x]` 已可用 / `[~]` 部分实现 / `[ ]` 未实现。

---

## 0. 现状速览

| 项目 | 路径 | 现状 |
|---|---|---|
| Java 应用 | `HMCL/src/main/java/org/jackhuang/hmcl` | 基准实现 |
| Java 核心库 | `HMCLCore/src/main/java/org/jackhuang/hmcl` | 基准实现 |
| C# 核心库 | `src/HMCLCore.NET` | 约 95%：账户/下载/实例/启动/整合包/附加内容/世界/主题/联机/统计均已实现，剩余为 Java 特有的 3D 皮肤预览 |
| C# 应用 | `src/HMCL.NET` | 约 90%：主框架 + 启动/账户/实例/下载/设置/日志窗口 + 向导 + 组件库 + i18n（10 语言）+ 主题（主题包/内置壁纸/配色风格） |
| Rust | `hmcl-rs/crates/hmcl-core` | 约 0%：仅 `util/operating_system.rs` |

---

## 1. Java 模块总览

### 1.1 HMCL 应用层（`HMCL/`）

| 包 | 职责 |
|---|---|
| `ui.main` | RootPage（主框架）、MainPage（启动页）、LauncherSettingsPage、SettingsPage、PersonalizationPage、DownloadSettingsPage、JavaManagementPage、JavaInstallPage、JavaDownloadDialog、JavaRestorePage、HelpPage、FeedbackPage、AboutPage、ThemePackManagementPage |
| `ui.account` | 账户列表/登录（Microsoft、离线、authlib-injector 服务器）、账户右键菜单、添加认证服务器 |
| `ui.instances` | 实例列表、实例详情、模组/资源包/数据包/世界/截图/蓝图/安装器、附加内容更新、世界备份与导出 |
| `ui.download` | 游戏版本安装向导、整合包安装向导、可选文件、本地整合包、更新安装器 |
| `ui.main`（设置子页） | 全局游戏设置、Java 管理、通用/外观/下载设置 |
| `ui.game` | GameSettingsPage（实例设置）、预设管理 |
| `ui.terracotta` | 联机（Terracotta）状态页 |
| `ui.wizard` | 向导框架（Navigation/Refreshable/Summary/TaskExecutor） |
| `ui.construct` | 组件库：AdvancedListBox、ClassTitle、LinePane、MessageDialogPane、TaskListPane、TabControl、PopupMenu、HintPane、Validators 等 |
| `ui.decorator` | 窗口装饰、页面转场、标题栏、窗口状态 |
| `ui.nbt` | NBT 编辑器与文件类型 |
| `ui.skin` | 皮肤库 |
| `ui.export` | 整合包导出向导（信息、类型、文件选择） |
| `theme` | 主题/背景/主题包/颜色提取 |
| `setting` | LauncherSettings、UserSettings、GameSettings、预设、账户元数据、认证服务器列表、代理、下载源、游戏目录 |
| `terracotta` | 联机元数据/节点/状态 |
| `upgrade` | 更新通道、更新检查、自更新、完整性校验 |
| `countly` | 匿名统计（可选项） |

### 1.2 HMCLCore 核心库（`HMCLCore/`）

| 包 | 关键内容 |
|---|---|
| `auth` | Account/AccountFactory/AccountID、ClassicAccount（离线）、OAuthAccount、Microsoft、Yggdrasil、AuthlibInjector、CharacterSelector、各类异常 |
| `download` | DownloadProvider、DownloadCandidate(s)、DependencyManager、GameBuilder、ComponentRemoteVersion(s) |
| `download.game` | 版本 JSON/客户端/库/资源索引/资源对象下载、完整性修复、远程版本列表 |
| `download.<loader>` | forge、neoforge、fabric、quilt、legacyfabric、optifine、liteloader、cleanroom 的远程版本与安装任务 |
| `download.java` | JavaDistribution、JavaPackageType、JavaRemoteVersion（Mojang/Disco） |
| `game` | GameRepository/Layout/Draft/Snapshot、GameInstance、Manifest、Patch、LibraryBuilder、AssetIndex、ExtractRules、CrashReportAnalyzer、World、QuickPlay 相关 |
| `launch` | Launcher、DefaultLauncher、CommandBuilder、ProcessListener、StreamPump、ExitWaiter、权限/长度/解压异常 |
| `modpack` | Modpack/Manifest/File/Configuration、Provider、Install/Update/Export、ModAdviser、Curse/Modrinth/MCBBS/MultiMC/Server |
| `addon` | AddonLoader(Type)、LocalAddonFile/Manager、RemoteAddon(Repository)、ProjectInfoRegistry |
| `task` | TaskExecutor、FileDownloadTask、FetchTask、CacheFileTask、TaskCompletableFuture |
| `util` | 平台（OS/Arch/版本/硬件）、io、i18n、logging、skin、versioning、NBT、Zlib、Digest、SettingsMap 等 |
| `event` | 事件总线 |
| `schematic` | 蓝图（schem/nbt/litematic） |

---

## 2. UI 布局规范（Java 基准）

### 2.1 主窗口与侧边栏（RootPage）

```
┌─────────────────────────────────────────────────────────────┐
│ [装饰标题栏] Hello Minecraft! Launcher        [最小化/最大化/关闭] │
├───────────────┬─────────────────────────────────────────────┤
│ 账户           │                                             │
│ ◉ 头像 用户名   │                                             │
│   账户类型      │              页面内容（切换/转场）              │
│               │                                             │
│ 实例           │                                             │
│ ▣ 图标 实例名   │                                             │
│   版本/加载器   │                                             │
│ ☰ 实例管理      │                                             │
│ ⬇ 下载          │                                             │
│               │                                             │
│ 设置           │                                             │
│ ⚙ 启动器设置    │                                             │
│ ◈ 联机          │                                             │
│               │                                             │
│ [内存状态条]    │                                             │
└───────────────┴─────────────────────────────────────────────┘
```
- 侧边栏宽度 180–400（默认 240），可拖拽分隔。
- 分类标题为全大写小字号；条目为两行（主标题 + 副标题/图标）。
- 页面为“栈式导航”：进入子页显示返回按钮；根页为启动页。
- 底部为物理内存状态条（已用/总计/分配）。

### 2.2 启动页（MainPage）
- 左上：图标 + 标题 `Hello Minecraft! Launcher`。
- 公告卡片（仅 nightly/dev 通道显示，可关闭）。
- 右上：更新气泡（230×55，可关闭；点击打开升级）。
- 右下：大号“启动游戏”按钮（上行为“启动游戏”，下行为当前实例名）+ 菜单按钮（实例选择弹窗）。
- 无实例时按钮文案“无游戏实例”，点击引导安装。
- 背景：主题背景（内置图/自定义图/纯色/网络图，可调不透明度）。
- 游戏日志：**独立 LogWindow**（启动时按设置显示）。

### 2.3 账户页（AccountListPage）
- 左栏（200px）：“添加账户”方法列表：Microsoft、离线、authlib-injector 服务器（可增删）；底部“添加认证服务器”。
- 中部：账户卡片列表（头像、用户名、类型、当前账户标记），右键菜单（设为当前/删除/上传皮肤等）。
- 只读账户文件时提示备份并覆盖。

### 2.4 实例列表页（GameListPage）
- 左栏（200px）：游戏目录列表 + “新建游戏目录”；底部“全局游戏设置”。
- 中部工具栏：刷新、安装新游戏、安装整合包、搜索（支持 `regex:` 前缀）。
- 列表项：图标、实例名、版本 + 加载器标签、最后游玩时间、右侧启动按钮。
- 右键菜单：启动、实例设置、复制、重命名、删除、导出整合包、打开文件夹等。

### 2.5 实例详情页（GameInstancePage）
- 头部：实例图标、名称、版本/加载器，操作按钮（启动测试、启动脚本、设置、重命名、复制、删除、导出整合包、更新、清理）。
- 标签页：**设置 / 安装器 / 模组 / 资源包 / 世界 / 蓝图 / 截图 / 文件夹**。
- 文件夹快捷入口：game/config/mod/resourcepacks/saves/schematics/shaderpacks/screenshots/logs/crash-reports。

### 2.6 下载页
- 左栏分类：游戏、整合包、模组、资源包、世界、数据包、光影。
- 游戏版本页（VersionsPage）：
  - 顶部搜索卡片：名称搜索 + 分类下拉（全部 / 正式版 / 快照版 / 愚人节版 / 旧版）。
  - 列表项：图标、版本名、发布时间、标签（正式/快照/旧版/愚人节）、支持状态告警、Wiki/安装按钮。
- 附加内容下载列表（DownloadListPage）：
  - 数据源切换（Modrinth / CurseForge）、搜索、分类、排序（相关度/下载量/更新时间/最新发布）、游戏版本与加载器过滤、分页。
  - 详情页（DownloadPage）：项目信息、版本列表按游戏版本分组、安装到实例/下载。
- 整合包安装向导：来源选择（本地文件/远程）、模组选择、可选文件、安装进度。
- 更新安装器（UpdateInstallerWizardProvider）。

### 2.7 设置页（LauncherSettingsPage）
- 左栏分类：
  - 全局游戏设置
  - Java 管理
  - **启动器**：通用、外观、下载
  - **帮助**：帮助、反馈、关于
- 通用：语言、禁用愚人节、调试、启动器日志（打开/导出）、其他。
- 外观：主题、亮度（自动/亮/暗）、主题色（默认/系统/自定义/背景提取 + 样式）、背景（类型/不透明度/缓存策略/回退）、标题栏透明、窗口透明、动画开关、界面字体、日志字体与字号。
- 下载：版本列表源、文件下载源、默认附加内容源、下载线程（自动/自定义）、代理（默认/无/HTTP/SOCKS + 主机/端口/认证）。
- Java 管理：本地 Java 列表（启用/禁用/移除）、添加、下载 Java（Mojang/Disco）、无效 Java 提示、只读目录处理。
- 帮助/反馈/关于：文档链接、反馈渠道、版本与依赖信息、开源许可。

### 2.8 全局/实例游戏设置（GameSettingsPage）
- 基本：Java 选择（自动/版本/自定义路径/检测结果）、内存（自动/最小/最大）、JVM 参数、窗口类型（窗口化/最大化/全屏）、运行目录、快捷进入（无/多人/单人/Realms）。
- 高级：游戏参数、环境变量、进程优先级、启动选项、`javaw` 开关、禁用 JVM 参数/优化参数/有效性检查、PermGen/元空间、自定义命令（包装器/启动前/退出后）、图形后端与渲染器、GPU 偏好、原生库（不自解压/自定义目录/GLFW/SDL/OpenAL）。
- 继承机制：全局预设 → 目录设置 → 实例覆盖（继承/覆盖）。
- 预设管理：新建/重命名/删除/设为默认。

### 2.9 其它页面
- 世界列表/管理/备份/导出（种子地图链接、数据包、锁定检测）。
- 截图页：网格、删除/复制/设为背景/刷新/打开目录/按日期筛选/全选。
- 蓝图页：类型识别（schem/nbt/litematic）、导入、目录创建、信息展示。
- 模组页：已安装列表（启用/禁用/删除）、本地添加、检查更新、搜索下载（仓库/分类/排序/版本过滤）、整合包模组更新告警。
- 资源包页：启用/禁用、添加、兼容性告警（过新/过旧/无效/缺 pack.mcmeta/缺 game meta）、检查更新。
- 数据包页：启用/禁用、添加、删除、重载提示。
- NBT 编辑器：树形查看/编辑/保存。
- 皮肤库：上传/选择/应用皮肤与披风。
- 联机（Terracotta）：状态、节点列表、加入/创建。
- 更新：检查更新、下载、自更新（可执行文件头处理）、完整性校验。

---

## 3. 任务清单（按域，含状态）

### A. 基础设施与平台

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| INF-01 | 平台识别（OS/Arch/版本） | `util/platform/*` | [x] | [~] | P0 |
| INF-02 | 硬件信息（内存/GPU） | `platform/hardware` | [x] | [ ] | P2 |
| INF-03 | 任务执行器/进度/取消 | `task/*` | [x] | [ ] | P0 |
| INF-04 | 文件下载（多候选/重试/校验/并发） | `FileDownloadTask` | [x] | [ ] | P0 |
| INF-05 | 缓存仓库（版本 JSON/库/资源） | `CacheRepository` | [x] | [ ] | P1 |
| INF-06 | 设置存储（JSON Schema / 迁移） | `setting/*` | [x] | [ ] | P0 |
| INF-07 | 国际化（i18n，多语言资源） | `util/i18n` | [x] | [ ] | P1 |
| INF-08 | 日志（文件/级别/导出） | `Log`/`LogExporter` | [x] | [ ] | P2 |
| INF-09 | 版本比较/排序（Minecraft/加载器） | `util/versioning` | [x] | [ ] | P0 |
| INF-10 | Zlib/NBT/摘要/字符串/IO 工具 | `util/*` | [x] | [ ] | P1 |
| INF-11 | 事件总线 | `event` | [x] | [ ] | P2 |
| INF-12 | 匿名统计（Countly，可关闭） | `countly` | [x] | [ ] | P3 |

### B. 账户与认证

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| AUTH-01 | 离线账户（UUID 派生/皮肤） | `ClassicAccount` | [x] | [ ] | P0 |
| AUTH-02 | Microsoft 完整链路（OAuth→XBL→XSTS→MC） | `auth/microsoft/*` | [x] | [ ] | P0 |
| AUTH-03 | Yggdrasil 认证 | `auth/yggdrasil/*` | [x] | [ ] | P0 |
| AUTH-04 | authlib-injector（jar/预取/参数注入） | `auth/authlibinjector/*` | [x] | [ ] | P0 |
| AUTH-05 | 认证服务器列表管理（增删/元数据缓存） | `AuthlibInjectorServers*` | [x] | [ ] | P1 |
| AUTH-06 | 账户元数据/私有数据存储 | `AccountMetadataStore` | [x] | [ ] | P1 |
| AUTH-07 | 角色选择（多角色账户） | `CharacterSelector` | [x] | [ ] | P2 |
| AUTH-08 | 账户只读文件检测与备份覆盖 | `SettingsManager` | [x] | [ ] | P2 |
| AUTH-09 | 皮肤/披风上传与缓存 | `game/TexturesLoader`, `ui/skin` | [x] | [ ] | P1 |

### C. 下载与安装

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| DL-01 | Mojang 版本清单/版本 JSON | `download/game/*` | [x] | [ ] | P0 |
| DL-02 | 客户端 JAR / 库 / 资源索引 / 资源对象 | `download/game/*` | [x] | [ ] | P0 |
| DL-03 | 完整性校验与修复 | `GameVerificationFixTask` | [x] | [ ] | P1 |
| DL-04 | 下载源（Mojang/BMCLAPI/镜像） | `DownloadProviders` | [x] | [ ] | P1 |
| DL-05 | Forge/NeoForge 安装 | `download/forge`, `neoforge` | [x] | [ ] | P0 |
| DL-06 | Fabric/Quilt/LegacyFabric（含 API） | `download/fabric|quilt|legacyfabric` | [x] | [ ] | P0 |
| DL-07 | OptiFine / LiteLoader / Cleanroom | 对应包 | [x] | [ ] | P2 |
| DL-08 | Java 运行时下载与安装（Mojang/Disco） | `download/java`, `HMCLJavaRepository` | [x] | [ ] | P1 |
| DL-09 | 下载线程数/并发控制 | `LauncherSettings` | [x] | [ ] | P2 |
| DL-10 | 加载器组件列表与安装器离线安装 | `GameComponentType` | [x] | [ ] | P1 |

### D. 实例与启动

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| GAME-01 | 仓库布局/草稿/快照 | `game/*Repository*` | [x] | [ ] | P0 |
| GAME-02 | 实例清单/补丁/库构建/规则 | `GameInstanceManifest`, `GameInstancePatch`, `GameInstanceLibraryBuilder` | [x] | [ ] | P0 |
| GAME-03 | 启动参数生成（JVM/游戏/内存/服务端） | `DefaultLauncher`, `CommandBuilder` | [x] | [ ] | P0 |
| GAME-04 | 进程输出流/退出等待/权限错误处理 | `launch/*` | [x] | [ ] | P1 |
| GAME-05 | 原生库解压/自定义原生库/GLFW/SDL/OpenAL | `ExtractRules`, `GameSettings` | [x] | [ ] | P1 |
| GAME-06 | 实例设置（继承/覆盖/预设） | `GameSettings`, `GameSettingsPresets` | [x] | [ ] | P0 |
| GAME-07 | 快捷进入（单人/多人/Realms） | `GameSettings.quickPlay*` | [x] | [ ] | P2 |
| GAME-08 | 环境变量/进程优先级/包装器/前后命令 | `GameSettings` | [x] | [ ] | P2 |
| GAME-09 | 实例重命名/复制/删除/图标 | `GameInstancePage`, `GameInstanceIconType` | [x] | [ ] | P1 |
| GAME-10 | 实例更新（版本升级） | `instances.update` | [x] | [ ] | P2 |
| GAME-11 | 崩溃报告分析/日志导出 | `CrashReportAnalyzer`, `LogExporter` | [x] | [ ] | P2 |
| GAME-12 | 启动脚本生成/启动测试 | `instance.launch_script` | [x] | [ ] | P3 |

### E. 内容管理

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| CONT-01 | 模组扫描/启用/禁用/删除 | `LocalAddonManager` | [x] | [ ] | P0 |
| CONT-02 | 资源包管理 + 兼容性告警 | `ResourcePackListPage` | [x] | [ ] | P1 |
| CONT-03 | 数据包管理 + 重载 | `DataPackListPage` | [x] | [ ] | P1 |
| CONT-04 | 光影管理 | `ShaderPackListPage`（Java 内嵌） | [x] | [ ] | P2 |
| CONT-05 | 世界列表/管理/备份/导出 | `World*Page`, `WorldBackupTask` | [x] | [ ] | P1 |
| CONT-06 | 截图管理（删除/复制/设为背景） | `ScreenshotsPage` | [x] | [ ] | P2 |
| CONT-07 | 蓝图管理（schem/nbt/litematic） | `SchematicsPage` | [x] | [ ] | P3 |
| CONT-08 | 附加内容更新检查/批量更新 | `AddonUpdatesPage` | [x] | [ ] | P1 |
| CONT-09 | 本地附加内容元数据解析（mod 名称/版本/加载器） | `AddonLoader` | [x] | [ ] | P1 |
| CONT-10 | 模组/资源包/数据包导入（文件选择/拖拽） | 各 ListPage | [x] | [ ] | P1 |

### F. 整合包

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| MP-01 | CurseForge / Modrinth 解析与安装 | `modpack/curse|modrinth` | [x] | [ ] | P0 |
| MP-02 | MCBBS / MultiMC / Server 格式 | `modpack/mcbbs|multimc|server` | [x] | [ ] | P2 |
| MP-03 | 整合包更新（版本差异/文件替换） | `ModpackUpdateTask` | [x] | [ ] | P1 |
| MP-04 | 整合包导出向导（信息/类型/文件选择） | `ui/export/*` | [x] | [ ] | P1 |
| MP-05 | 模组建议/可选文件 | `ModAdviser`, `OptionalFilesPage` | [x] | [ ] | P2 |
| MP-06 | 安装进度/失败回滚/未解析模组报告 | 各 Task | [x] | [ ] | P1 |

### G. 设置与外观

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| SET-01 | 启动器通用设置（语言/愚人节/调试/日志） | `SettingsPage` | [x] | [ ] | P1 |
| SET-02 | 外观设置（主题/亮度/主题色/背景/透明/动画/字体） | `PersonalizationPage`, `theme/*` | [x] | [ ] | P1 |
| SET-03 | 下载设置（源/线程/代理含 SOCKS） | `DownloadSettingsPage` | [x] | [ ] | P1 |
| SET-04 | Java 管理页（列表/启用/禁用/卸载/下载） | `JavaManagementPage` | [x] | [ ] | P1 |
| SET-05 | 全局游戏设置页（预设 + 实例两层继承） | `GameSettingsPage` | [x] | [ ] | P0 |
| SET-06 | 游戏目录管理（多目录/新增/编辑） | `GameDirectoryPage` | [x] | [ ] | P1 |
| SET-07 | 设置预设与继承 | `GameSettingsPresets` | [x] | [ ] | P1 |
| SET-08 | 设置迁移（旧版配置迁移） | `LegacyConfigMigrator` | [x] | [ ] | P3 |
| SET-09 | 主题包管理（导入/导出） | `ThemePackManager` | [x] | [ ] | P3 |

### H. 应用级功能

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| APP-01 | 更新检查（通道/预览/跳过） | `UpdateChecker` | [x] | [ ] | P1 |
| APP-02 | 自更新（下载/替换/完整性） | `UpdateHandler`, `HMCLDownloadTask` | [x] | [ ] | P2 |
| APP-03 | 联机 Terracotta（状态/节点/加入） | `terracotta/*`, `ui/terracotta` | [x] | [ ] | P3 |
| APP-04 | NBT 编辑器 | `ui/nbt` | [x] | [ ] | P2 |
| APP-05 | 帮助/反馈/关于页 | `HelpPage`, `FeedbackPage`, `AboutPage` | [x] | [ ] | P2 |
| APP-06 | 公告（nightly/dev） | `MainPage` | [x] | [ ] | P3 |
| APP-07 | 窗口状态记忆（尺寸/位置/最大化） | `WindowBounds` | [x] | [ ] | P2 |
| APP-08 | 托盘/后台行为 | 平台相关 | [x] | [ ] | P3 |

### I. UI 组件库与交互

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| UI-01 | 向导框架（导航/刷新/摘要/任务执行） | `ui/wizard/*` | [x] | [ ] | P1 |
| UI-02 | 对话框体系（消息/输入/确认/任务/进度） | `construct/*DialogPane` | [x] | [ ] | P1 |
| UI-03 | 高级列表/分类标题/两行条目 | `AdvancedListBox`, `ClassTitle` | [x] | [ ] | P1 |
| UI-04 | 行式设置控件（文本/选择/开关/文件/滑块） | `Line*` | [x] | [ ] | P1 |
| UI-05 | 弹出菜单/右键菜单/上下文菜单 | `PopupMenu` | [x] | [ ] | P1 |
| UI-06 | 任务列表/进度面板 | `TaskListPane` | [x] | [ ] | P1 |
| UI-07 | 提示条/Toast/骨架屏/加载失败页 | `HintPane`, `SpinnerPane` | [x] | [ ] | P2 |
| UI-08 | 页面转场动画/波纹/加载动画 | `ui/animation`, `RipplerContainer` | [x] | [ ] | P3 |
| UI-09 | 无障碍/键盘（Esc/快捷键） | `FXUtils` | [x] | [ ] | P3 |

---

### J. 细化补项（2026-10 复审新增）

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| EXTRA-01 | 启动器可见性（游戏启动后隐藏/关闭/重开） | `GameSettings.launcherVisibility` | [x] | [ ] | P1 |
| EXTRA-02 | 世界信息页（难度/模式/玩家/种子/出生点/最后死亡） | `ui/world/WorldInfoPage` | [x] | [ ] | P2 |
| EXTRA-03 | 联机大厅（邀请码/玩家列表/网络难度/房主访客流程） | `ui/terracotta/*` | [x] | [ ] | P3 |
| EXTRA-04 | 下载页安装器目录（按游戏版本列出全部安装器与版本） | `ui/download/InstallersPage` | [x] | [ ] | P2 |
| EXTRA-05 | 全局预设管理（新建/重命名/删除/实例继承覆盖） | `settings.type.global.preset` | [x] | [ ] | P2 |
| EXTRA-06 | 主题细节（内置壁纸库/配色风格/动画开关/字体抗锯齿/主题色板） | `theme/*` | [x] | [ ] | P3 |
| EXTRA-07 | 设置文件「备份并覆盖」（只读/高版本文件） | `settings.file.force_write` | [x] | [ ] | P2 |
| EXTRA-08 | NBT 格式兼容（gzip/复合键名/列表无名/Minecraft 格式） | `util/nbt` | [x] | [ ] | P1 |
| EXTRA-09 | 七语言人工翻译（ja/ru/uk/de/es/ar/lzh 的 C# 自有文案） | `assets/lang` | [x] | [ ] | P3 |
| EXTRA-10 | 世界图标编辑/世界复制/难度锁定 | `ui/world/*` | [x] | [ ] | P3 |
| EXTRA-11 | 联机组件本地安装（拖入安装包安装） | `terracotta.from_local` | [x] | [ ] | P3 |
| EXTRA-12 | 匿名统计服务端配置（默认关闭） | `countly/*` | [x] | [ ] | P3 |
| EXTRA-13 | 主题资源包体系（资源/清单/作者/条件覆盖/内置壁纸取色） | `theme/ThemePack*` | [x] | [ ] | P3 |
| EXTRA-14 | 更新检查仓库目标修正（`KnightBlood/HMCL` 当前无 release） | `UpdateChecker` | [ ] | [ ] | P2 |
| EXTRA-15 | 主题包管理页（列出/导入/删除/应用主题包） | `ThemePackManagementPage` | [x] | [ ] | P3 |
| EXTRA-16 | 世界备份页（备份列表/新建/删除/在文件夹中显示） | `WorldBackupsPage` | [x] | [ ] | P3 |
| EXTRA-17 | 崩溃原因分析（规则匹配 + 日志窗口展示崩溃原因） | `CrashReportAnalyzer` | [x] | [ ] | P2 |
| EXTRA-18 | 整合包可选文件（服务器端文件可选安装） | `OptionalFilesPage` | [x] | [ ] | P3 |
| EXTRA-19 | 联机页反馈/日志导出 + 按服务状态切换按钮 | TerracottaControllerPage | [x] | [ ] | P3 |
| EXTRA-20 | 窗口蓝色标题栏（返回/标题/帮助，可拖拽）+ 默认壁纸背景 + 侧栏分组与空状态 | `ui/decorator/MainWindowPane` `ui/main/RootPage` | [x] | [ ] | P2 |
| EXTRA-21 | 下载页按 Java 版布局（搜索/版本类型/刷新/版本行标签与图标/加载失败态） | `ui/download/VersionsPage` | [x] | [ ] | P2 |
| EXTRA-22 | 设置页按 Java 版布局（左侧导航 + 分节卡片式行 + 顶部保存按钮） | `ui/main/LauncherSettingsPage` | [x] | [ ] | P2 |
| EXTRA-23 | 原生 AOT 发布（免 .NET 运行时，源码生成 JSON、编译期绑定） | `build.gradle.kts` 的 shadowJar | [x] | [ ] | P2 |
| EXTRA-20 | 世界占用检测（session.lock，备份/删除/复制时拒绝操作） | `World.isLocked` | [x] | [ ] | P2 |
| EXTRA-21 | 实例游戏设置完整覆盖项（Java/窗口/全屏/优先级/可见性/图形后端/包装器/前后命令） | `GameSettingsPage` | [x] | [ ] | P2 |
| EXTRA-22 | 启动器崩溃报告窗口（复制/发送报告/系统信息） | `CrashWindow`, `CrashReporter` | [x] | [ ] | P2 |
| EXTRA-23 | 游戏日志窗口开关（全局与实例级） | `GameSettings.showLogs` | [x] | [ ] | P3 |
| EXTRA-24 | 高级启动选项（禁用版本 JVM 参数/PermGen/自定义原生库目录/Java 版本检查） | `GameSettingsPage` 高级 | [x] | [ ] | P2 |

> EXTRA-14 按用户要求暂缓处理；其余细化补项已于 2026-10 完成（C# 侧）。

## 4. C# 执行计划

### K. 全目录复审（2026-10-22，含 HMCLBoot/ 与 minecraft/）

> 上一轮审计只覆盖了 `HMCL/` 与 `HMCLCore/` 两个目录，遗漏了 `minecraft/libraries/`。
> 本节按四个目录重新核对，并**下调**了此前标记过头的行（`[~]` = 核心能力可用，但与 Java 版仍有可见差距）。

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| K-01 | 启动清单修复（LaunchWrapper/ModLauncher/BootstrapLauncher、LegacyFabric ASM 冲突、重复库、旧 log4j 补丁） | `LaunchManifestNormalizer` | [x] | [ ] | P0 |
| K-02 | 内置 Java 库注入（MultiMC 引导器 + 转换器发现服务） | `minecraft/libraries/*` | [x] | [ ] | P0 |
| K-03 | MultiMC 实例设置在安装后应用到实例（Java/内存/命令/窗口/控制台） | `MultiMCInstancePatch` | [x] | [ ] | P2 |
| K-04 | 远程整合包浏览与安装（CurseForge/Modrinth） | `RemoteModpackPage` | [x] | [ ] | P2 |
| K-05 | 附加内容工程信息注册表（项目元数据/条件） | `AddonProjectInfoRegistry` | [x] | [ ] | P2 |
| K-06 | 内置网页浏览（远程内容/整合包详情页） | `WebPage` | [x] | [ ] | P3 |
| K-07 | 3D 皮肤模型预览（旋转/动画） | `ui/skin/*` | [x] | [ ] | P3 |
| K-08 | 游戏崩溃转储 | `GameDumpGenerator` | [x] | [ ] | P3 |
| K-09 | 启动器日志字体/字号与日志入口 | `PersonalizationPage` | [x] | [ ] | P3 |
| K-10 | 向导摘要页（Summary） | `ui/wizard/Summary` | [x] | [ ] | P3 |
| K-11 | 游戏目录的「使用相对路径」与目录级设置 | `GameDirectoryPage` | [x] | [ ] | P3 |

> `HMCLBoot/`（Java 版启动引导）经核对**不适用**于 .NET 实现（其职责是 JVM 引导、外观与编码），无需移植；
> 但其中的 `boot.manual_update`（自动更新不可用时引导手动下载）在 C# 侧尚无对应提示。


### 阶段 0：工程与基础设施（1–2 周）
1. 引入 i18n 资源体系（`zh_CN`/`en`），替换硬编码中文。
2. 设置模型升级：`LauncherSettings` 补齐字段（语言、更新通道、下载线程、代理类型、主题、背景、字体、日志行数、游戏目录等）+ JSON Schema/迁移。
3. 统一任务与下载队列：并发/重试/候选源/取消/进度事件。
4. 日志体系：启动器日志文件、级别、导出。
5. 代理全类型（HTTP/SOCKS4/SOCKS5）与全局应用。
6. 验收：设置可持久化并生效；下载支持并发与取消；日志可导出。

### 阶段 1：账户与认证补全（1–2 周）
1. authlib-injector 服务器列表管理（增删/改名/元数据缓存/只读检测）。
2. 账户元数据/私有数据迁移。
3. 角色选择（多角色）。
4. 头像/皮肤缓存完善（离线 UUID 派生、失败回退）。
5. 验收：添加/删除认证服务器后，登录与启动参数正确。

### 阶段 2：下载与安装补全（3–4 周）
1. 加载器：Forge/NeoForge/Fabric/Quilt/LegacyFabric/OptiFine/LiteLoader/Cleanroom 远程版本与安装任务。
2. Java 运行时下载（Mojang/Disco）与本地 Java 管理页。
3. 完整性校验与修复（版本 JSON/库/资源）。
4. 下载源切换（版本列表/文件）与镜像重写完善。
5. 验收：可从零安装 1.20.1 Fabric/Forge 实例并启动。

### 阶段 3：实例设置与启动（2–3 周）
1. GameSettings（继承/覆盖/预设）：Java 选择、内存、JVM 参数、窗口类型、快捷进入、环境变量、优先级、包装器/前后命令、原生库与图形后端。
2. 实例复制/重命名/删除/图标/更新。
3. 启动脚本生成、启动测试。
4. 验收：实例设置覆盖全局并正确进入启动命令；预设可切换。

### 阶段 4：内容管理（3–4 周）
1. 资源包/数据包/光影页（启用/禁用/导入/告警/更新）。
2. 世界列表/管理/备份/导出/数据包。
3. 截图页（删除/复制/设为背景/筛选）。
4. 蓝图页（类型识别/导入/目录）。
5. 附加内容更新（批量检查/更新/导出）。
6. 验收：各内容页可完成常规操作并通过启动验证。

### 阶段 5：整合包与导出（1–2 周）
1. MCBBS/MultiMC/Server 格式。
2. 整合包更新任务。
3. 导出向导（信息/类型/文件选择）。
4. 模组建议/可选文件。
5. 验收：导出整合包可被 Java 版与 C# 版重新安装。

### 阶段 6：应用外观与 UI 组件库（3–4 周）
1. 主题/背景/颜色提取/字体/动画。
2. 向导框架与实例详情页（标签页结构）。
3. 对话框/任务列表/Toast/右键菜单统一组件。
4. 更新检查/自更新、NBT 编辑器、帮助/反馈/关于。
5. 验收：UI 布局与 Java 版逐页一致；关键流程均有向导与进度反馈。

### 阶段 7（可选）：联机与统计
1. Terracotta 联机页与状态机。
2. Countly（默认关闭）。

---

## 5. Rust 执行计划

### 目标架构（建议）
```
hmcl-rs/
├── Cargo.toml                # workspace
├── crates/
│   ├── hmcl-core/            # 平台、任务、设置、i18n、日志、util、版本比较
│   ├── hmcl-auth/            # 离线/Microsoft/Yggdrasil/authlib-injector
│   ├── hmcl-download/        # providers、游戏/加载器/Java 下载
│   ├── hmcl-game/            # 仓库、实例、清单、库、资源、世界、崩溃分析
│   ├── hmcl-launch/          # 命令构建、进程管理、流泵
│   ├── hmcl-modpack/         # curse/modrinth/mcbbs/multimc/server、导入导出
│   ├── hmcl-addon/           # 本地/远程附加内容
│   └── hmcl-app/             # UI（Tauri/Slint/egui 任选）+ 页面
```

### 阶段 R0：核心骨架（1–2 周）
- `OperatingSystem`/`Architecture` 补全；错误类型（`thiserror`）；异步运行时（tokio）与任务取消（`CancellationToken`）。
- 设置存储（serde + JSON Schema 校验 + 迁移）；日志（tracing）。
- i18n（fluent 或 rust-i18n）。
- 验收：核心 crate 可编译、单测覆盖平台/设置/版本比较。

### 阶段 R1：认证（2 周）
- 离线 UUID 派生、Microsoft OAuth/XBL/XSTS/Minecraft、Yggdrasil、authlib-injector（jar 下载/预取/参数）。
- 令牌刷新与持久化（`keyring` 可选）。
- 验收：三类账户可登录并生成启动参数。

### 阶段 R2：下载与游戏核心（3–4 周）
- 版本清单/JSON/客户端/库/资源；校验与修复。
- 加载器安装（优先 Fabric/Forge）。
- Java 运行时发现与下载。
- 仓库/实例/清单/库构建/规则/原生库。
- 验收：CLI 可安装并启动 1.20.1 Fabric 实例。

### 阶段 R3：启动与整合包（2–3 周）
- 命令构建、进程/流泵/退出等待。
- CurseForge/Modrinth 整合包安装；导出。
- 验收：CLI 安装整合包并启动。

### 阶段 R4：附加内容与设置（2–3 周）
- 本地附加内容扫描/启用/删除；Modrinth/CurseForge 搜索与安装。
- 游戏设置（继承/预设）、代理、下载源。
- 验收：模组可搜索安装并生效。

### 阶段 R5：UI（4–6 周）
- 选定 UI 技术（建议 Tauri + Web 前端或 Slint），按第 2 节布局实现：
  主框架/侧边栏 → 启动页/日志窗口 → 账户页 → 实例列表/详情 → 下载分类/版本/附加内容 → 设置分类页 → 向导/对话框。
- 验收：页面结构与交互对标 Java 版；关键流程可用。

### 阶段 R6：应用功能（2–3 周）
- 更新检查/自更新、NBT、截图/世界/蓝图、Terracotta（可选）、统计（可选）。

---

## 6. 依赖关系与优先级

- P0（必须先做）：INF-01/03/04/06/09、AUTH-01/02/03/04、DL-01/02/05/06、GAME-01/02/03/06、CONT-01、MP-01、SET-05。
- P1：INF-05/07/10、AUTH-05/06/09、DL-03/04/08/10、GAME-04/05/09/11、CONT-02/03/05/08/09/10、MP-03/04/06、SET-01/02/03/04/06/07、APP-01、UI-01/02/03/04/05/06。
- P2：INF-02/08/11、AUTH-07/08、DL-07/09、GAME-07/08/10、CONT-04/06、MP-02/05、SET-08、APP-02/04/05/07、UI-07。
- P3：INF-12、CONT-07、GAME-12、SET-09、APP-03/06/08、UI-08/09。

关键依赖：
- `GAME-06`（实例设置）依赖 `INF-06`（设置模型）。
- `DL-05/06`（加载器）依赖 `DL-01/02`（版本/库/资源）与 `GAME-02`（清单/补丁）。
- `CONT-08`（更新）依赖 `CONT-09`（元数据解析）与远程仓库。
- `UI-01`（向导）依赖 `UI-02/03/04`（对话框/列表/行控件）。
- 主题（`SET-02`）依赖 `theme` 模块与资源加载。

---

## 7. 交付标准（Definition of Done）

1. 功能与 Java 版对应类/流程一致，异常与边界情况有明确提示。
2. 所有新增 C#/Rust 代码通过编译且 0 警告（C# 遵循 `AGENTS.md`：`@NotNullByDefault`/显式 `@Nullable`/`///` 文档/不可变集合注解）。
3. UI 页面结构与第 2 节布局一致；关键操作有进度与失败反馈。
4. 关键流程有冒烟测试（安装版本、安装加载器、安装整合包、安装模组、启动、备份/还原、导出）。
5. 网络与文件操作具备超时/重试/取消；不阻塞 UI 线程。
6. GPLv3 合规：衍生实现保持 GPLv3，保留版权与许可声明。

---

## 8. 风险与注意事项

- **许可**：HMCL 为 GPLv3；C#/Rust 移植与分发需遵循 GPLv3。
- **认证安全**：Microsoft/Yggdrasil 令牌需安全存储；日志中不得输出令牌。
- **第三方 API**：CurseForge 需 API Key；Modrinth 有速率限制；需缓存与退避。
- **下载源**：镜像可能失效；需多候选与回退。
- **跨平台**：Windows 路径/权限/`javaw`、macOS/Linux 差异需平台抽象。
- **UI 一致性**：Java 使用 JavaFX 自定义控件；复刻时优先保证布局层级与交互语义一致，而非像素级。
