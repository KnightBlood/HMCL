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
| Rust | `hmcl-rs/crates/hmcl-core` | 约 25%：平台/架构、任务、设置存储、i18n、日志、事件总线、版本比较、摘要/压缩/NBT |

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
| INF-01 | 平台识别（OS/Arch/版本） | `util/platform/*` | [x] | [x] | P0 |
| INF-02 | 硬件信息（内存/GPU） | `platform/hardware` | [x] | [~] | P2 |
| INF-03 | 任务执行器/进度/取消 | `task/*` | [x] | [~] | P0 |
| INF-04 | 文件下载（多候选/重试/校验/并发） | `FileDownloadTask` | [x] | [~] | P0 |
| INF-05 | 缓存仓库（版本 JSON/库/资源） | `CacheRepository` | [x] | [~] | P1 |
| INF-06 | 设置存储（JSON Schema / 迁移） | `setting/*` | [x] | [~] | P0 |
| INF-07 | 国际化（i18n，多语言资源） | `util/i18n` | [x] | [~] | P1 |
| INF-08 | 日志（文件/级别/导出） | `Log`/`LogExporter` | [x] | [x] | P2 |
| INF-09 | 版本比较/排序（Minecraft/加载器） | `util/versioning` | [x] | [~] | P0 |
| INF-10 | Zlib/NBT/摘要/字符串/IO 工具 | `util/*` | [x] | [~] | P1 |
| INF-11 | 事件总线 | `event` | [x] | [~] | P2 |
| INF-12 | 匿名统计（Countly，可关闭） | `countly` | [x] | [ ] | P3 |

### B. 账户与认证

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| AUTH-01 | 离线账户（UUID 派生/皮肤） | `ClassicAccount` | [x] | [~] | P0 |
| AUTH-02 | Microsoft 完整链路（OAuth→XBL→XSTS→MC） | `auth/microsoft/*` | [x] | [~] | P0 |
| AUTH-03 | Yggdrasil 认证 | `auth/yggdrasil/*` | [x] | [~] | P0 |
| AUTH-04 | authlib-injector（jar/预取/参数注入） | `auth/authlibinjector/*` | [x] | [~] | P0 |
| AUTH-05 | 认证服务器列表管理（增删/元数据缓存） | `AuthlibInjectorServers*` | [x] | [x] | P1 |
| AUTH-06 | 账户元数据/私有数据存储 | `AccountMetadataStore` | [x] | [x] | P1 |
| AUTH-07 | 角色选择（多角色账户） | `CharacterSelector` | [x] | [ ] | P2 |
| AUTH-08 | 账户只读文件检测与备份覆盖 | `SettingsManager` | [x] | [x] | P2 |
| AUTH-09 | 皮肤/披风上传与缓存 | `game/TexturesLoader`, `ui/skin` | [x] | [ ] | P1 |

### C. 下载与安装

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| DL-01 | Mojang 版本清单/版本 JSON | `download/game/*` | [x] | [~] | P0 |
| DL-02 | 客户端 JAR / 库 / 资源索引 / 资源对象 | `download/game/*` | [x] | [~] | P0 |
| DL-03 | 完整性校验与修复 | `GameVerificationFixTask` | [x] | [~] | P1 |
| DL-04 | 下载源（Mojang/BMCLAPI/镜像） | `DownloadProviders` | [x] | [x] | P1 |
| DL-05 | Forge/NeoForge 安装 | `download/forge`, `neoforge` | [x] | [ ] | P0 |
| DL-06 | Fabric/Quilt/LegacyFabric（含 API） | `download/fabric|quilt|legacyfabric` | [x] | [ ] | P0 |
| DL-07 | OptiFine / LiteLoader / Cleanroom | 对应包 | [x] | [ ] | P2 |
| DL-08 | Java 运行时下载与安装（Mojang/Disco） | `download/java`, `HMCLJavaRepository` | [x] | [ ] | P1 |
| DL-09 | 下载线程数/并发控制 | `LauncherSettings` | [x] | [~] | P2 |
| DL-10 | 加载器组件列表与安装器离线安装 | `GameComponentType` | [x] | [ ] | P1 |

### D. 实例与启动

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| GAME-01 | 仓库布局/草稿/快照 | `game/*Repository*` | [x] | [~] | P0 |
| GAME-02 | 实例清单/补丁/库构建/规则 | `GameInstanceManifest`, `GameInstancePatch`, `GameInstanceLibraryBuilder` | [x] | [x] | P0 |
| GAME-03 | 启动参数生成（JVM/游戏/内存/服务端） | `DefaultLauncher`, `CommandBuilder` | [x] | [x] | P0 |
| GAME-04 | 进程输出流/退出等待/权限错误处理 | `launch/*` | [x] | [~] | P1 |
| GAME-05 | 原生库解压/自定义原生库/GLFW/SDL/OpenAL | `ExtractRules`, `GameSettings` | [x] | [~] | P1 |
| GAME-06 | 实例设置（继承/覆盖/预设） | `GameSettings`, `GameSettingsPresets` | [x] | [x] | P0 |
| GAME-07 | 快捷进入（单人/多人/Realms） | `GameSettings.quickPlay*` | [x] | [x] | P2 |
| GAME-08 | 环境变量/进程优先级/包装器/前后命令 | `GameSettings` | [x] | [~] | P2 |
| GAME-09 | 实例重命名/复制/删除/图标 | `GameInstancePage`, `GameInstanceIconType` | [x] | [~] | P1 |
| GAME-10 | 实例更新（版本升级） | `instances.update` | [x] | [ ] | P2 |
| GAME-11 | 崩溃报告分析/日志导出 | `CrashReportAnalyzer`, `LogExporter` | [x] | [~] | P2 |
| GAME-12 | 启动脚本生成/启动测试 | `instance.launch_script` | [x] | [x] | P3 |

### E. 内容管理

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| CONT-01 | 模组扫描/启用/禁用/删除 | `LocalAddonManager` | [x] | [~] | P0 |
| CONT-02 | 资源包管理 + 兼容性告警 | `ResourcePackListPage` | [x] | [~] | P1 |
| CONT-03 | 数据包管理 + 重载 | `DataPackListPage` | [x] | [~] | P1 |
| CONT-04 | 光影管理 | `ShaderPackListPage`（Java 内嵌） | [x] | [~] | P2 |
| CONT-05 | 世界列表/管理/备份/导出 | `World*Page`, `WorldBackupTask` | [x] | [~] | P1 |
| CONT-06 | 截图管理（删除/复制/设为背景） | `ScreenshotsPage` | [x] | [x] | P2 |
| CONT-07 | 蓝图管理（schem/nbt/litematic） | `SchematicsPage` | [x] | [~] | P3 |
| CONT-08 | 附加内容更新检查/批量更新 | `AddonUpdatesPage` | [x] | [~] | P1 |
| CONT-09 | 本地附加内容元数据解析（mod 名称/版本/加载器） | `AddonLoader` | [x] | [x] | P1 |
| CONT-10 | 模组/资源包/数据包导入（文件选择/拖拽） | 各 ListPage | [x] | [~] | P1 |

### F. 整合包

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| MP-01 | CurseForge / Modrinth 解析与安装 | `modpack/curse|modrinth` | [x] | [~] | P0 |
| MP-02 | MCBBS / MultiMC / Server 格式 | `modpack/mcbbs|multimc|server` | [x] | [x] | P2 |
| MP-03 | 整合包更新（版本差异/文件替换） | `ModpackUpdateTask` | [x] | [x] | P1 |
| MP-04 | 整合包导出向导（信息/类型/文件选择） | `ui/export/*` | [x] | [x] | P1 |
| MP-05 | 模组建议/可选文件 | `ModAdviser`, `OptionalFilesPage` | [x] | [x] | P2 |
| MP-06 | 安装进度/失败回滚/未解析模组报告 | 各 Task | [x] | [x] | P1 |

### G. 设置与外观

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| SET-01 | 启动器通用设置（语言/愚人节/调试/日志） | `SettingsPage` | [x] | [ ] | P1 |
| SET-02 | 外观设置（主题/亮度/主题色/背景/透明/动画/字体） | `PersonalizationPage`, `theme/*` | [x] | [ ] | P1 |
| SET-03 | 下载设置（源/线程/代理含 SOCKS） | `DownloadSettingsPage` | [x] | [~] | P1 |
| SET-04 | Java 管理页（列表/启用/禁用/卸载/下载） | `JavaManagementPage` | [x] | [~] | P1 |
| SET-05 | 全局游戏设置页（预设 + 实例两层继承） | `GameSettingsPage` | [x] | [ ] | P0 |
| SET-06 | 游戏目录管理（多目录/新增/编辑） | `GameDirectoryPage` | [x] | [ ] | P1 |
| SET-07 | 设置预设与继承 | `GameSettingsPresets` | [x] | [x] | P1 |
| SET-08 | 设置迁移（旧版配置迁移） | `LegacyConfigMigrator` | [x] | [x] | P3 |
| SET-09 | 主题包管理（导入/导出） | `ThemePackManager` | [x] | [x] | P3 |

### H. 应用级功能

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| APP-01 | 更新检查（通道/预览/跳过） | `UpdateChecker` | [x] | [ ] | P1 |
| APP-02 | 自更新（下载/替换/完整性） | `UpdateHandler`, `HMCLDownloadTask` | [x] | [ ] | P2 |
| APP-03 | 联机 Terracotta（状态/节点/加入） | `terracotta/*`, `ui/terracotta` | [x] | [ ] | P3 |
| APP-04 | NBT 编辑器 | `ui/nbt` | [x] | [~] | P2 |
| APP-05 | 帮助/反馈/关于页 | `HelpPage`, `FeedbackPage`, `AboutPage` | [x] | [ ] | P2 |
| APP-06 | 公告（nightly/dev） | `MainPage` | [x] | [ ] | P3 |
| APP-07 | 窗口状态记忆（尺寸/位置/最大化） | `WindowBounds` | [x] | [x] | P2 |
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
| EXTRA-02 | 世界信息页（难度/模式/玩家/种子/出生点/最后死亡） | `ui/world/WorldInfoPage` | [x] | [~] | P2 |
| EXTRA-03 | 联机大厅（邀请码/玩家列表/网络难度/房主访客流程） | `ui/terracotta/*` | [x] | [ ] | P3 |
| EXTRA-04 | 下载页安装器目录（按游戏版本列出全部安装器与版本） | `ui/download/InstallersPage` | [x] | [ ] | P2 |
| EXTRA-05 | 全局预设管理（新建/重命名/删除/实例继承覆盖） | `settings.type.global.preset` | [x] | [ ] | P2 |
| EXTRA-06 | 主题细节（内置壁纸库/配色风格/动画开关/字体抗锯齿/主题色板） | `theme/*` | [x] | [ ] | P3 |
| EXTRA-07 | 设置文件「备份并覆盖」（只读/高版本文件） | `settings.file.force_write` | [x] | [~] | P2 |
| EXTRA-08 | NBT 格式兼容（gzip/复合键名/列表无名/Minecraft 格式） | `util/nbt` | [x] | [~] | P1 |
| EXTRA-09 | 七语言人工翻译（ja/ru/uk/de/es/ar/lzh 的 C# 自有文案） | `assets/lang` | [x] | [ ] | P3 |
| EXTRA-10 | 世界图标编辑/世界复制/难度锁定 | `ui/world/*` | [x] | [~] | P3 |
| EXTRA-11 | 联机组件本地安装（拖入安装包安装） | `terracotta.from_local` | [x] | [ ] | P3 |
| EXTRA-12 | 匿名统计服务端配置（默认关闭） | `countly/*` | [x] | [ ] | P3 |
| EXTRA-13 | 主题资源包体系（资源/清单/作者/条件覆盖/内置壁纸取色） | `theme/ThemePack*` | [x] | [x] | P3 |
| EXTRA-14 | 更新检查仓库目标修正（`KnightBlood/HMCL` 当前无 release） | `UpdateChecker` | [ ] | [ ] | P2 |
| EXTRA-15 | 主题包管理页（列出/导入/删除/应用主题包） | `ThemePackManagementPage` | [x] | [~] | P3 |
| EXTRA-16 | 世界备份页（备份列表/新建/删除/在文件夹中显示） | `WorldBackupsPage` | [x] | [ ] | P3 |
| EXTRA-17 | 崩溃原因分析（规则匹配 + 日志窗口展示崩溃原因） | `CrashReportAnalyzer` | [x] | [~] | P2 |
| EXTRA-18 | 整合包可选文件（服务器端文件可选安装） | `OptionalFilesPage` | [x] | [ ] | P3 |
| EXTRA-19 | 联机页反馈/日志导出 + 按服务状态切换按钮 | TerracottaControllerPage | [x] | [ ] | P3 |
| EXTRA-20 | 窗口蓝色标题栏（返回/标题/帮助，可拖拽）+ 默认壁纸背景 + 侧栏分组与空状态 | `ui/decorator/MainWindowPane` `ui/main/RootPage` | [x] | [ ] | P2 |
| EXTRA-21 | 下载页按 Java 版布局（搜索/版本类型/刷新/版本行标签与图标/加载失败态） | `ui/download/VersionsPage` | [x] | [ ] | P2 |
| EXTRA-22 | 设置页按 Java 版布局（左侧导航 + 分节卡片式行 + 顶部保存按钮） | `ui/main/LauncherSettingsPage` | [x] | [ ] | P2 |
| EXTRA-23 | 原生 AOT 发布（免 .NET 运行时，源码生成 JSON、编译期绑定） | `build.gradle.kts` 的 shadowJar | [x] | [ ] | P2 |
| EXTRA-20 | 世界占用检测（session.lock，备份/删除/复制时拒绝操作） | `World.isLocked` | [x] | [~] | P2 |
| EXTRA-21 | 实例游戏设置完整覆盖项（Java/窗口/全屏/优先级/可见性/图形后端/包装器/前后命令） | `GameSettingsPage` | [x] | [ ] | P2 |
| EXTRA-22 | 启动器崩溃报告窗口（复制/发送报告/系统信息） | `CrashWindow`, `CrashReporter` | [x] | [ ] | P2 |
| EXTRA-23 | 游戏日志窗口开关（全局与实例级） | `GameSettings.showLogs` | [x] | [ ] | P3 |
| EXTRA-24 | 高级启动选项（禁用版本 JVM 参数/PermGen/自定义原生库目录/Java 版本检查） | `GameSettingsPage` 高级 | [x] | [x] | P2 |

> EXTRA-14 按用户要求暂缓处理；其余细化补项已于 2026-10 完成（C# 侧）。

## 4. C# 执行计划

### K. 全目录复审（2026-10-22，含 HMCLBoot/ 与 minecraft/）

> 上一轮审计只覆盖了 `HMCL/` 与 `HMCLCore/` 两个目录，遗漏了 `minecraft/libraries/`。
> 本节按四个目录重新核对，并**下调**了此前标记过头的行（`[~]` = 核心能力可用，但与 Java 版仍有可见差距）。

| ID | 任务 | Java 参考 | C# | Rust | 优先级 |
|---|---|---|---|---|---|
| K-01 | 启动清单修复（LaunchWrapper/ModLauncher/BootstrapLauncher、LegacyFabric ASM 冲突、重复库、旧 log4j 补丁） | `LaunchManifestNormalizer` | [x] | [x] | P0 |
| K-02 | 内置 Java 库注入（MultiMC 引导器 + 转换器发现服务） | `minecraft/libraries/*` | [x] | [~] | P0 |
| K-03 | MultiMC 实例设置在安装后应用到实例（Java/内存/命令/窗口/控制台） | `MultiMCInstancePatch` | [x] | [x] | P2 |
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

**进度（Rust）**
- 已完成：
  - `util/operating_system.rs`：OS/Arch/`SystemInfo`（含物理内存，Windows 用 `GlobalMemoryStatusEx`、Linux 用 `/proc/meminfo`）；
  - `error.rs`：错误类型（IO/格式/版本/不支持，带文件路径）；
  - `task/`：任务、取消令牌、进度、`RunningTask`（后台线程执行 + 进度收集）；
  - `setting/`：`LauncherSettings`/`UserSettings`（字段与 Java 的 JSON Schema 一致，未知字段保留）、
    `schema.rs`（schema 版本兼容检查：同版本可写、较新版本只读、较旧版本迁移并改写地址）、
    `storage.rs`（原子读写、损坏文件在下次保存时编号备份为 `*.json.1`/`*.json.2`…、
    无法解析的文件用默认值继续启动、其他类型的文件只读不覆盖）；
  - `i18n/`：Java `.properties` 解析器 + 10 种语言资源 + 英文回退 + `%s`/`%1$s` 占位符；
  - `util/logging.rs`：stderr 日志 + 日志文件（按启动时刻命名、保留份数、导出、访问令牌脱敏）；
  - `util/datetime.rs`：本地时间与 UTC 时间（Windows 用 `SystemTimeToTzSpecificLocalTime`，Unix 用 `localtime_r`）；
  - `event/`：事件总线（按类型分通道、优先级、可取消事件、事件结果）；
  - `util/versioning/`：`VersionNumber`（Maven 规范）、`GameVersionNumber`（含 `versions.txt`/`version-alias.csv` 资源）、
    `VersionRange`；语义与 Java 一致，并由 Java 单测移植的 Rust 单测覆盖；
  - `util/digest.rs`：SHA-1/SHA-256/SHA-512/MD5（手写实现，摘要与 Java 一致）；
  - `util/compression.rs`：zlib/gzip/deflate 压缩解压，并用与 Java 相同的测试向量校验 zlib 兼容性；
  - `download/game/`：Mojang 版本清单（`VersionManifest`）与版本 JSON（`VersionMetadata`：库/下载/资产索引/
    兼容性规则/参数）的模型，与 Java 的 `GameRemoteVersions`/`GameInstancePatch` 字段一致（DL-01 的模型部分）；
    库坐标 `Artifact`（`group:name:version[:classifier][@ext]`、文件名与路径规则）与实例清单
    `InstanceManifest`（GAME-02 的模型部分）；
  - `auth/`：账户标识 `AccountId`（`account-` 前缀 + UUIDv7）、`Uuid`（解析/格式化，离线玩家 UUID=MD5(`OfflinePlayer:<name>`)）、
    `AuthInfo` 与账户模型（离线 / Microsoft / authlib-injector）、`accounts.json` 公开元数据读写（AUTH-01/AUTH-06 的模型部分）；
  - `addon/remote_addon.rs`：远端插件（来源 CurseForge/Modrinth、类型、版本、依赖、文件校验按 sha512→md5 选择）；
  - `setting/game_directory.rs`：游戏目录（自定义名称与相对路径解析）；
  - `game/repository_layout.rs`：官方目录布局（`versions/<id>`、`libraries`、`assets/indexes`、`assets/objects/<前两位>`、
    `log_configs`、`mods`/`resourcepacks`/`shaderpacks`/`saves`/`logs`/`crash-reports`/`screenshots`）与实例本地库规则
    （GAME-01 的路径部分）。

应用层新增显示模型（供页面绑定）：`models/auth.rs`（`HMCL.Auth.Account`：Username/SkinPath/AccountType）、
`models/download.rs`（`HMCL.Download.VersionMetadata`：Id/Type/ReleaseTimeText）、`models/remote_addon.rs`
（`HMCL.Addon.RemoteAddon`：Name/Description/Author/Slug/PageUrl/IconUrl）、`models/setting.rs`
（`HMCL.Setting.GameDirectory`）。

- **实例解析与组件识别（GAME-02 续）**：
  - `download/game/instance_patch.rs`：补丁模型（`priority`，以及对库/参数/主类/资产/Java 版本/下载等字段的覆盖）；
  - `InstanceManifest::merge` / `apply_patch`、`Arguments::merge`：子清单覆盖标量字段，父参数在前、子参数在后，
    库按“子在前、父在后”拼接，`minimumLauncherVersion` 取大者；
  - `game/resolver.rs`：`InstanceResolver` 沿 `inheritsFrom` 链递归解析，缺失父版本报错、循环继承安全终止，
    补丁按 `priority` 升序应用，`jar` 缺省回落到实例 id；
  - `game/component.rs`：`GameComponentType`（12 种：game / fabric / legacyfabric(+api) / forge / cleanroom /
    neoforge / liteloader / optifine / quilt(+api)）与 `ComponentAnalyzer`（补丁优先、库特征兜底、
    Forge/NeoForge 版本提取、`is_modded` 与 ModLauncher 判定）。

- **启动命令生成（GAME-03 首版）**：
  - `download/game/version_metadata.rs`：`Argument` 支持 `value`/`values` 为字符串或数组（`ArgumentValue`），
    规则求值新增 `RuleContext` + `rules_allow`（OS 名称/版本/架构 + features，默认 DISALLOW、按顺序应用），
    OS 名称/架构规范化（含 `win-x86` 这类合写）与轻量正则（锚点、`.`、字符类、`*`/`+`/`?`）；
  - `game/launch/`：
    - `variables.rs`：`LaunchVariables` 与 `${...}` 替换（未知变量保持原样），覆盖账号、版本、资产、
      类路径、库/natives 目录、启动器名与语言等变量；
    - `arguments.rs`：参数解析（规则过滤 + 变量替换）、旧格式 `minecraftArguments` 分词（支持引号）、
      默认 JVM 参数（natives 路径、`-cp`、启动器标识、Windows 兼容参数）与默认游戏参数（自定义分辨率）；
    - `classpath.rs`：类路径收集（跳过 native 库与其他系统的库）、`;`/`:` 分隔符、native classifier 推导
      （`natives-windows-${arch}` 与 `<os>-<bits>` 等写法）；
    - `command.rs`：`build_command` 生成 `LaunchCommand`（JVM 参数、主类、游戏参数、类路径），支持内存/
      元空间、玩家附加参数、全屏与关闭默认参数；`rule_context`/`arch_name` 供启动准备层复用。

- **启动准备层（GAME-03 / DL-02 续）**：
  - `util/zip.rs`：手写 ZIP 读取器（EOCD 反向搜索、中央目录、本地文件头、stored/deflate、zip64 extra），
    路径穿越（`..`/绝对路径/盘符）拒绝；用仓库内真实 jar 覆盖读取与解压；
  - `download/game/asset_index.rs`：资产索引 `AssetIndexFile`（`virtual`/`map_to_resources`/`objects`）与
    `AssetObject`（按哈希前两位分目录的 `location()`、`ASSET_URL` 下载地址）；
  - `DownloadInfo` 新增 `path`，库文件优先使用版本声明的路径；
  - `game/launch/libraries.rs`：`unique_libraries` 复刻 Java `LaunchManifestNormalizer` 的去重语义——按
    `group:artifact` + 规则分组，版本高者胜，同版本同内容取元数据更全者，同版本不同文件（如 jar 与 natives）保留两条；
  - `game/launch/library.rs`：`LibraryFile`（路径/URL/SHA-1/大小/classifier/是否 native），旧格式库回落到
    `url + path`，本地库（无下载地址）不产生下载任务；
  - `game/launch/required.rs`：必需文件清单（客户端、库与 native 库、资产索引、资产对象）、
    `missing_files`（存在性 + 大小）与 `verify_file`（SHA-1 校验，用于客户端 JAR 修复）；
  - `game/launch/natives.rs`：`NativeArchive`（归档、校验、`extract.exclude`）、`extract_natives`
    与 `extract_natives_with`（解压前清空目标目录由计划负责）；解压过滤对齐 Java `DefaultLauncher.decompressNatives`：
    跳过符号链接、`.sha1`/`.git` 元数据、目标目录中大小相同的已有文件，`NativeOptions` 可跳过 GLFW/SDL 与 OpenAL
    （玩家使用系统库时）；
  - `game/launch/assets.rs`：`virtual_assets` 决定旧版本（`virtual`）按名称读取资产时的目录布局
    （`assets/virtual/<id>` 与可选的 `resources/`），`VirtualAssets::copy` 在下载完成后按名称复制对象，
    拒绝 `..`/绝对路径/盘符等越界名称；
  - `game/launch/plan.rs`：`plan_launch` 汇总 `LaunchPlan`（命令、必需/缺失文件、native 归档、目录、
    `assets_directory` 与 `virtual_assets`）；旧版本的 `${game_assets}`/`${assets_root}` 指向虚拟资产目录，
    `prepare()` 创建目录并清空 natives 目录，`prepare_assets()` 在下载后复制虚拟资产，`is_ready()` 判断是否可离线启动；
  - `util/zip.rs` 读取中央目录的 Unix 权限位，`ZipEntry::is_symlink` 供 natives 过滤使用；
  - `game/launch/service.rs`：`FileDownloader` 下载扩展点（由应用层实现）、`LaunchService::prepare`
    （建目录 → 下载缺失文件 → 复制虚拟资产 → 解压 natives）与 `start_command`/`LaunchService::start`
    （工作目录、玩家环境变量、`GameProcess` 接管子进程输出）；
  - `game/launch/process.rs`：`pump_lines` 逐行读取标准输出/标准错误（去掉行尾、无效 UTF-8 用替换字符），
    `GameProcess` 在后台线程读取输出并回调（可实时写入日志窗口），`wait` 合并输出后按 Java `ExitWaiter`
    分类 `ExitType`（JVM_ERROR / APPLICATION_ERROR / SIGKILL / NORMAL / INTERRUPTED）：
    JVM 启动失败标记需非零退出码，崩溃报告等标记在 0 退出码下也算应用错误，
    Linux/BSD 的 137 识别为被系统杀死；启动命令无权限时返回 `ProcessError::PermissionDenied`；
  - `game/launch/normalize.rs` 对齐 Java `LaunchManifestNormalizer.repairForLaunch`（K-01）：
    拒绝未解析版本（`inherits_from` 仍在）与仍有补丁的版本；先去重库，再按主类分派——
    `LaunchWrapper` 恢复 LiteLoader / OptiFine / Forge 的 `--tweakClass`（重新排序到参数最前，
    仅当对应组件仍在时），Forge 走 ModLauncher 时把主类换成 ModLauncher 并去掉 LaunchWrapper 的 tweaker；
    `ModLauncher` 且 Forge + OptiFine 时补 `-Dhmcl.transformer.candidates=${library_directory}/<OptiFine 路径>`
    并追加 `org.jackhuang.hmcl:transformer-discovery-service:1.0`（幂等）；
    `BootstrapLauncher` 在引导器版本 ≥ 0.1.17 时把 `${primary_jar_name}` 加进 `-DignoreList=`；
    LegacyFabric 有新版 `org.ow2.asm:asm` 时删掉 `asm-all`；首库为 `org.glavo:log4j-patch(-beta9):1.0` 时删除；
    旧格式版本的 `minecraftArguments` 保持单串（`arguments` 置空、token 以空格拼接，对齐 Java `addAllWithoutParsing`）；
    `plan_launch` 在版本已解析时自动调用它，未解析版本按原清单继续计划；
  - `InstanceResolver` 解析后清空 `patches`（对齐 Java `DefaultGameRepositorySnapshot` 的 `builder.setPatches(null)`），
    组件分析改由合并后的库推断；
  - 说明：下载执行器仍待接入 HTTP 客户端（离线环境缺少可用的 TLS crate），当前只产出下载清单与校验信息。
- **崩溃报告分析（GAME-11 首版）**：
  - `game/crash_report.rs`：移植 Java `CrashReportAnalyzer` 的全部 59 条规则（OpenJ9/32 位 JVM/Java 版本过低或过高/内存不足/显卡驱动/
    Forge/Fabric 模组解析/重复安装/模组崩溃/OptiFine 冲突等），正则保留 Java 写法，编译前转换为 Rust `regex`
    （`(?<name>` → `(?P<name>`、`\R` → 显式换行），按规则顺序返回首个匹配及其命名分组；
  - `find_crash_report` 读取日志中 `#@!@# Game crashed! Crash report saved to: #@!@#` 指向的报告，
    `extract_crash_report` 从日志中截取崩溃报告，`find_keywords_from_crash_report` 提取可能属于模组的包名
    （排除游戏/Java/加载器包名），`java_version_from_major_version` 把 class 文件版本换算为 Java 版本；
  - 测试直接复用 Java 仓库的 `HMCLCore/src/test/resources` 日志样本，覆盖规则匹配、命名分组、关键词提取与报告定位；
  - 离线环境已可使用 `regex` crate（registry 缓存完整），后续可按需替换早期手写的轻量正则。
- **Java 运行时发现与选择（SET-04 / GAME-06 的 Java 部分）**：
  - `java/mod.rs`：`Platform`（OS + 架构；`Display` 与 Java `Platform.toString()` 的目录名一致，如 `windows-x86-64`）、
    release 文件的 OS/架构名解析（`parse_os_name`/`parse_arch_name`）、可执行文件名，以及跨架构兼容判定
    （Windows/Linux x64 可运行 x86、Windows ARM64 可运行 x64/x86、macOS ARM64 可运行 x64）；
  - `java/info.rs`：`JavaInfo` 读取 `release` 文件（`JAVA_VERSION`/`OS_NAME`/`OS_ARCH`/`IMPLEMENTOR`）或
    `java -XshowSettings:properties -version` 输出（`java.version`/`java.vendor`/`os.arch`），解析主版本
    （`1.8.0_462`→8）、`VersionNumber` 比较、厂商名归一化；
  - `java/runtime.rs`：`JavaRuntime::detect` 规范化为真实路径并读取信息、校验平台兼容、识别 JDK（`javac`），
    读取无响应的运行时 10 秒后终止；排序为“托管优先 → 主版本升序 → 版本号 → 架构 → 路径”；
  - `java/discovery.rs`：`JavaSearch` 扫描 Java home（`<dir>/<name>`）、已知 home、可执行文件、HMCL 托管仓库
    （`<root>/<platform>/<name>.json`）与官方启动器运行时（`<root>/<component>/<platform>/<component>`），
    去重并跳过禁用项；`default_search` 覆盖 `JAVA_HOME`/`HMCL_JRES`/`PATH`、Windows 厂商目录与 Minecraft 运行时、
    Linux/macOS 标准目录与 `~/.jdks`；
  - `game/java_version.rs`：`GameJavaVersion`（各游戏版本的最低 Java、Cleanroom 版本映射、按平台可下载的版本），
    `JavaVersionConstraint` 的 13 条规则（Vanilla/GameJson/Forge 各代 Java 7/8/16/17/21/Cleanroom/LaunchWrapper/
    1.8.0_51/Linux 32 位库/ARM64 x86/ModLauncher JDK-8273826）与 `find_suitable_java`
    （强制/建议规则分离、托管与版本优先、ARM64 旧版本强制 x86）；
  - 尚未接入：Java 管理页 UI、启用/禁用/卸载、Java 运行时下载（DL-08）。
- **实例读取与游戏版本识别（GAME-01 / GAME-02 续）**：
  - `util/zip.rs`：新增 `read_entry`/`read_entry_from`，按名称读取归档内的单个文件（复用中央目录与 deflate 逻辑），
    供版本识别与后续资源读取使用；`extract`/`extract_from` 抽出 `extract_under`，并新增
    `extract_under_directory`（剥离指定前缀后解压，世界压缩包的单层子目录依赖它）；
  - `util/zip.rs`：`ZipWriter::{create,add_file,add_path,add_directory,add_directory_filtered,finish}`
    写出 ZIP（条目按文件名排序，能变小才 deflate 否则 stored，先写 `.zip.part` 再改名，
    反斜杠统一为 `/`，自带 `crc32`），`WrittenEntry` 记录目录项；
  - `game/game_version.rs`：`minecraft_version` 按 Java `GameVersion` 的顺序识别版本——`version.json` 的 `id`
    （`"1.20.1 / 1.20.2"` 取前一个）、客户端类常量 `Minecraft Minecraft <version>`（`Beta`/`Alpha v`
    转写为 `b`/`a` 前缀，`RC1`/`RC2` 继续读服务端类）、服务端类中 `Can't keep up!` 之前最近的含数字字符串；
    自带 Java class 常量池解析（Utf8/String、long/double 占两槽）；旧版本的 SHA-1 已知版本表尚未移植；
  - `game/repository.rs`：`GameRepository` 扫描 `versions/<id>`，优先读 `<id>.json`，否则读取目录内唯一的 `*.json`，
    以目录名作为实例 id（不重命名磁盘文件），跳过无清单/多清单目录，损坏清单记录为 `RepositoryError` 并继续；
    `jar_file` 由清单文件同名 `.jar` 推导，`version` 由 `minecraft_version` 识别，
    `resolve` 复用 `InstanceResolver` 应用继承链与补丁；
  - `game/repository_snapshot.rs`：`RepositorySnapshot` 是仓库的不可变索引（清单、清单文件、已识别版本），
    `GameRepository::snapshot()` 返回当前快照；`refresh` 整体替换快照而非就地修改，
    读多个实例的一方可以持有一份快照，界面列出的实例与随后读到的清单因此保持一致；
  - `game/repository_draft.rs`：对齐 Java `DefaultGameRepositoryDraft`
    （`GameRepository::open_draft` 借出仓库，编译期保证同一时间只有一次写会话）：
    `put`/`remove`/`rename`/`putPrimaryJar` 只改内存，`commit` 时才创建缺少的实例目录、
    复制客户端 jar、写清单、移动/删除实例目录，最后一次性发布新快照；
    rename 同步更新直接继承者的 `inheritsFrom`、把 `jar == from` 改为引用自身并把已识别版本随目录迁移，
    非约定清单文件名在改名后保持原名（`manifest_files` 记到新 id 下）；
    commit 失败按「文件 → 被移除目录 → 改名」的逆序回滚，`state` 记为 `Failed` 并释放会话；
    `abort`/Drop 删除本次草稿创建的目录，`put` 拒绝占用草稿开启前就已存在的未注册目录；
    临时文件放在 `<game>/.hmcl/repository-drafts/commit-*`；
  - `GameRepository::{save,remove_instance_from_disk,rename,delete}` 改用草稿语义：
    `save` 与 `rename` 一次提交发布；`remove_instance_from_disk` 对已注册实例走草稿，
    对未注册孤儿目录沿用「改名为 `<id>_removed` 后删除」并刷新（不依赖系统回收站）；
    `refresh` 后剔除继承链缺父实例的条目（循环继承保留，由 resolver 在第二次访问时停止），
    对齐 Java `removeInvalidInstances`；`copy` 仍为直接复制（Java 亦无草稿版本）；
  - 说明：仓库的读取/写入与快照/草稿已实现（GAME-01），实例图标（GAME-09）也已移植到 `game/instance_icon.rs`。
- **下载缓存与文件下载任务（INF-04 / INF-05 核心）**：
  - `download/cache.rs` 的 `CacheRepository` 对齐 Java `CacheRepository`：文件按摘要布局为
    `<common>/cache/<算法>/<摘要前两位>/<摘要>`（摘要小写，读写都先校验摘要，被改动过的文件不会被采用）；
    本地文件可通过 `check_existent_file` 回灌缓存（`cache_file`/`try_cache_file`/`copy_from_cache`）；
  - ETag 索引写在 `<common>/cache/etag.json`，字段与 Java 一致（`url`/`eTag`/`hash`/`local`/`remote`/`expires`）；
    键是去掉 query 与 fragment 的地址（`drop_query`）；无 `ETag`、或 `Cache-Control: no-store` 时不写条目，
    有 `max-age`/`s-maxage` 按秒计算过期，只有 `Expires` 时解析 HTTP 日期，两者都缺时取 `i64::MAX`；
    替换条目时若摘要不同会删掉旧缓存文件，索引写入走临时文件再改名，读到坏索引按空处理；
  - `download/task.rs` 的 `FileDownloadTask` 对齐 Java `FileDownloadTask` 的决策层：`DownloadCandidates`
    按顺序尝试多个镜像，`DownloadCandidate` 可单独指定重试次数（默认 5 次、间隔 200ms）；
    文件写到同目录的 `.<文件名>.part`，通过摘要校验（`IntegrityCheck`，SHA-1 等）与可选的归档校验
    （`util::zip::entries` 能打开）后才改名到目标位置，失败会删除临时文件；
  - 缓存优先：有摘要且启用缓存时先查 `CacheRepository`，命中则复制到目标；否则在启用远端缓存时带
    `If-None-Match` 发送条件请求，`304` 时把缓存文件复制到目标，`304` 但缓存缺失会去掉条件头重试；
    下载成功后按需 `cache_file`（按摘要）与 `cache_remote_file`（按 URL + ETag）；
  - `DownloadError` 统一了下载失败（网络/404/状态码/摘要/归档/IO/取消），`is_retryable()` 决定是否重试
    （`NotFound`、`Cancelled`、程序自报的 `Failed` 不重试）；`game::launch::DownloadError` 现改为复用该类型；
  - `download/downloader.rs` 的 `TaskDownloader` 把 `RequiredFile` 接到任务上（客户端与库做归档校验、
    资源文件不做），实现 `FileDownloader` 供 `LaunchService` 使用，缓存在下载器内部；`fetch_library`
    走库缓存后再下载，下载成功回灌库缓存；
  - `download/library_cache.rs` 的 `DefaultCacheRepository` 对齐 Java `DefaultCacheRepository`：
    库索引写在 `<common>/cache/index.json`（字段 `libraries[] = {name, hash, type}`，`type` 为 `jar`/`forge`），
    读到坏索引按空处理；`library_file` 先按摘要查缓存，再按库名查索引（`forge` 类型额外验 `checksums`），
    最后查旧版共享目录 `<common>/libraries/<path>` 并回灌缓存；
    `checksum_valid` 对齐 `LibraryDownloadTask.checksumValid`：先比整包 SHA-1，再读 jar 内 `checksums.sha1`
    （先校验清单本身的摘要，再逐行比文件摘要）；
  - `game/verification.rs` 对齐 Java `GameVerificationFixTask` 与 `GameLibrariesTask`：
    `remove_legacy_forge_signatures` 在游戏版本 `< 1.6` 且检测到 Forge 时，经新增的 `util::zip::remove_entries`
    重写客户端 jar，删除 `META-INF/MOJANG_C.DSA` 与 `META-INF/MOJANG_C.SF`；
    `library_needs_download` 对齐 `shouldDownloadLibrary`（文件缺失必下载 → `integrity_check` 关闭即跳过 →
    比 `sha1` → 验库 `checksums` → `.jar` 必须能被 `util::zip` 打开），
    `libraries_to_download` 把该判断接到实例清单上，产出需重下的 `RequiredFile` 列表（同为 `Library` 种类）；
    `fix_optifine_of_forge` 对齐 1.20.4 OptiFine + Forge 48.0.0–49.0.50 区间删除 `META-INF/mods.toml`；
    `fml_libraries` 返回 Forge `7.8.1.x`（1.5.2）缺失的 6 个 FML 库（名/摘要/`hmcl.glavo.site` 地址），
    `bundled_library_resource` 给出 `mmc-bootstrap` 与 `transformer-discovery-service` 的内置 jar 资源路径；
    `LaunchService::prepare_verified` 在下载完成后调用签名修复（组件未知时跳过）；
  - 完整性校验接入启动：`LaunchOptions::integrity_check`（对应 Java `LauncherHelper` 里由
    `unmarkLaunchedAbnormally()` 决定的 `integrityCheck`）打开时，`plan_launch` 会把 `libraries_to_download`
    找到的坏库并入 `missing_files`，因此它们会被重新下载；`GameRepository::mark_launched_abnormally` /
    `unmark_launched_abnormally` 读写实例目录下的 `.abnormal` 标记，供应用层决定是否打开该开关；
  - `game/launch/repair.rs` 对齐 `GameLibrariesTask.run()` 的修复段：`repair_libraries` 遍历生效库，
    先做 OptiFine 修复（按清单里 `optifine:OptiFine:1.20.4_*` 与组件的 Forge 版本判断），
    再按 `bundled_library_resource` 复制内置 jar（bootstrap 已存在则跳过、transformer 每次覆盖），
    最后用 `library_needs_download` 收集待下载项；OptiFine 组且没有下载地址的库不下载
    （对应 Java `library.hasDownloadURL() || !"optifine".equals(groupId)`）；
    `LaunchService::prepare_repaired` 把修复结果接到 `FileDownloader` 上，供应用层在启动前调用；
  - `download/provider.rs` 对齐 Java `DownloadProvider` / `HMCLDownloadProvider` / `DownloadCandidates`（DL-04）：
    `DownloadProvider` 维护 24 条 `MirrorRule`（Mojang 元数据与资源、`libraries.minecraft.net`、Forge/NeoForge/
    LiteLoader/Fabric 仓库、authlib-injector、Maven Central→腾讯云、Cleanroom/FML 库/未列出清单→8mi 镜像，
    以及 Modrinth/CurseForge 接口与 CDN 的 `fallback` 规则）；
    `candidates()` 按「是否中国大陆机器 × `SourcePreference::{Default,Official,Mirror}`」决定镜像优先级：
    大陆 `Default` 镜像优先、`Official` 官方优先、非大陆 `Default` 完全不用镜像、`Mirror` 镜像优先；
    `fallback` 规则即便镜像优先也把官方地址放在前面（模组仓库接口返回结构不同）；
    未命中规则的地址只保留一条；`asset_object_candidates`、`game_version_list_candidates`、
    `fabric_version_list_candidates`、`cleanroom_version_list_candidates`（仅大陆附带 8mi 镜像）分别给出清单地址；
    `is_china_mainland_machine(zone, utc_offset, country)` 复刻 Java `RegionUtils.checkMachineIsChinaMainland`
    （`Asia/Shanghai`，或 UTC+8 且国家为 `CN`）；镜像根末尾的 `/` 会被去掉；
  - 已实现：真实 HTTP/TLS `Fetcher` 在应用 crate 的 `http_fetcher.rs`，
    `hmcl-core` 只留协议纯逻辑（重定向语义、超时、代理、响应头，见下方 HTTP 说明）；
    仍缺下载任务并发与缺失报告。
    `DefaultCacheRepository` 尚无并发锁（Java 用读写锁，Rust 侧由调用方串行化）；
    FML 库（1.5.2）的下载尚未接到启动流程，只提供了地址表；内置 jar 的内容需由应用层以
    `read_resource("/assets/game/...")` 提供给 `repair_libraries`。
- **NBT 读写与编辑器（EXTRA-08 / APP-04 核心）**：
  - `util/nbt.rs`：`Tag::read` 读取并丢弃根标签名、`Tag::write` 写空根名（`[10,0,0,0]` 即空 compound 的完整字节），
    与游戏写出的文件一致；文本按 Java 的 modified UTF-8 编解码（含 `U+0000` 写作 `C0 80`、增补平面写成代理对），
    长度字段是**字节数**而非字符数——此前按字符递增，仓库里带中文区域名的 `<litematic>` 会被读成空 compound；
  - `read_compressed` 按魔数自动识别 gzip（`1f 8b`）/ zlib（`78`）/未压缩，`write_compressed` 统一写 gzip
    并先写 `.<文件名>.tmp` 再改名（`level.dat` 与 `level.dat_old` 不再共用临时文件）；
  - 编辑器显示模型：`display_value()`（按标签类型渲染要显示的文本）、`child_count()`、`is_leaf()`、`children()`；
  - `util/nbt_file.rs`：`NbtFileType` 按扩展名区分 `Compressed`（`dat`/`dat_old`/`litematic`/`nbt`/`schematic`/`schem`）
    与 `Anvil`/`Region`（`mca`/`mcr`），`read`/`write` 处理普通 NBT 文件，`read_region`/`write_region` 自行解析
    `.mca`/`.mcr`：按 8 KiB 扇区定位 `(x, z)` 块，支持压缩类型 1（gzip）/2（zlib）/3（未压缩），类型 4（LZ4）
    返回不支持；`NbtRegion`/`RegionChunk` 提供块计数与 `Chunk (x, z)` 显示名，写入固定用 zlib 并先写临时文件；
  - 尚未接入：NBT 编辑器的保存 UI（Java 侧保存也是 TODO），Rust 侧 `write_region` 已可用。
- **世界读取与管理（CONT-05 首版）**：
  - `game/world.rs`：`World::open` 读取 `saves/<name>/level.dat`（gzip 压缩的 NBT；快照 20w14infinite 的
    `special_level.dat` 也支持），校验 `Data`/`LevelName`/`LastPlayed`；
  - 只读信息对齐 Java `World`：`world_name`、`last_played`、`game_version`（`Version.Name`）、
    `seed`（1.16+ 读 `WorldGenSettings.seed`，之前读 `RandomSeed`）、`is_large_biomes`
    （1.16 前 `generatorName == largeBiomes`，1.16–1.18 `biome_source.large_biomes`，
    1.18+ `settings == minecraft:large_biomes`）、`support_data_packs`（1.13+）、`support_quick_play`（1.20+）；
  - `World::rename` 先改写 `level.dat` 的 `LevelName` 再重命名目录（与 Java 顺序一致），
    `World::copy` 跳过 `session.lock` 后按新名字重写等级数据，`World::delete` 删除目录；三者都拒绝已锁定
    （`session.lock` 无法写入）的世界；
  - `get_worlds(saves_dir)` 列出可读世界，非世界目录跳过；`WorldError` 区分
    `NotAWorld`/`Level`/`WorldLocked`/`RenameFailed`/`CopyFailed`/`DeleteFailed`；
  - `util/nbt.rs` 新增 `Tag::as_compound_mut`、`write_compressed`（写临时文件后改名，避免半写文件）；
  - `World::export(archive, world_name)`：用 `util::zip::ZipWriter` 把关目录写到 `<world_name>/` 下
    （跳过 `session.lock`，空名回落到目录名），`World::install(source, saves_dir, name)` 接受目录或 zip，
    zip 的等级数据既可在根也可在唯一子目录中（对齐 Java `World.install` 的两种布局），
    安装后把 `LevelName` 改写为安装名；目标已存在或压缩包无 `level.dat` 时拒绝并清理半成品；
  - `World::icon()` 读取世界目录下的 `icon.png` 原始字节，供应用层解码；
  - 世界信息页（EXTRA-02/EXTRA-10 核心）：`World` 额外读取 `data/minecraft/world_gen_settings.dat`
    与 `players/data/<uuid>.dat`（`singleplayer_uuid` 为 4 个 int 时按大端拼 UUID），
    `world_gen_settings()`/`player_data()` 统一返回 `WorldGenSettings` 与玩家复合标签；
    只读信息 `difficulty`/`is_difficulty_locked`/`allow_commands`/`generate_features`/
    `hardcore`/`game_mode`/`spawn_point`/`player_location`/`last_death_location`/
    `player_spawn_point`/`player_health`/`player_food_level`/`player_food_saturation`/
    `player_experience_level` 对齐 Java `WorldInfoPage` 的读取分支（1.21.9 前 `SpawnX/Y/Z`
    与之后 `spawn.pos`、1.16 前 `MapFeatures` 与之后 `generate_features`/`generate_structures`、
    26.1 后 `difficulty_settings`）；
    写回 `set_difficulty`/`set_difficulty_locked`/`set_allow_commands`/`set_generate_features`/
    `set_game_mode`（硬核写为生存 + `hardcore` 标志）/`set_player_*`，
    按世界实际保存的位置写回（等级、独立的世界生成设置文件、独立的玩家文件），
    `write_level_data()`/`write_world_data()` 对齐 Java 的写入顺序；
    `set_icon(image)` 只接受 64×64 的 PNG（读 `IHDR` 校验尺寸），`clear_icon()` 幂等删除 `icon.png`；
    `Dimension`/`WorldPosition`/`WorldLocation`/`Difficulty`/`GameMode` 提供展示与转换语义；
  - 尚未接入：世界信息页与世界图标编辑的 UI（Rust 侧仅核心逻辑）。
  - `game/world_backup.rs`：`backup(world, backups_dir)` 写出 `yyyy-MM-dd_HH-mm-ss_<世界目录名>[ n].zip`
    （同秒重名依次编号，最多 256 次，跳过 `session.lock`），`list_backups(dir, world_name)` 只识别本启动器
    写出的名字、按时间与编号升序，`Backup` 提供时间/编号/世界名/游戏版本并可从压缩包内的 `level.dat` 读回，
    `delete()` 幂等；`BackupError` 区分 `NotAWorld`/`WorldLocked`/`Write`/`Archive`/`TooManyAttempts`；
  - 尚未接入：难度/模式/玩家/出生点信息页（EXTRA-02）、世界复制/图标编辑 UI（EXTRA-10）。
- **资源包格式版本（`pack_format`）**：
  - `pack_version_of` 保留无 jar 时的版本表回退；
  - `pack_version_of_jar` 对齐 Java `ResourcePackManager.getPackVersion`：先按
    `13w24a`/`15w31a`/`16w32a`/`17w48a`/`18w47b` 判断旧版格式号，之后读游戏 jar 的
    `version.json`——`pack_version` 为对象时取 `resource_major`/`resource_minor`（>64 用新格式，
    否则回退 `resource`），为数字时用该数字；jar 缺失/无该文件时回退到 `pack_version_of`；
  - `PackManager::with_game_jar` 让管理器按 jar 读取格式，未设置时仍按版本表。
- **截图与蓝图（CONT-06/CONT-07 核心）**：
  - `screenshot.rs`：`Screenshot::is_file_screenshot`（仅 `.png`，大小写不敏感）与 `Screenshot::open`
    读取文件名/创建时间（Unix 上回退修改时间）/大小，`list_screenshots` 按路径倒序（最新在前）、
    跳过非 PNG 与目录，`clear_screenshots` 只删除截图、保留其他文件，`ScreenShotError::Delete`；
  - `schematic.rs`：`SchematicType`（`litematic`/`nbt`/`schematic`+`schem`，按扩展名大小写不敏感识别），
    `Schematic::open` 按扩展名分派，读取 Litematica（`Version`/`Metadata` 必需、`Regions` 计数、
    `EnclosingSize`、`SubVersion`、`MinecraftDataVersion`、`PreviewImageData`、`TotalBlocks`/`TotalVolume`、
    `TimeCreated`/`TimeModified`）、Sponge 新旧两版（`Materials` 判旧版、`Version` 及嵌套 `Schematic` 判新版、
    缺 `DataVersion` 取 1631、`Width`/`Height`/`Length` 按无符号 short 还原）、结构文件
    （`blocks`/`entities`/三项 `size` 必需，缺 `DataVersion` 取 500）；对外提供版本、名称、作者、描述、
    尺寸/体积、区域数、方块数、预览图像素与边长；
  - `schematics_directory(run_dir)` 读取 `config/litematica.json` 的 `Generic.customSchematicBaseDirectory`
    （相对路径按游戏目录解析、路径规范化），`list_schematics` 目录在前、按名字排序，
    `import_schematic` 拒绝覆盖同名文件，`create_schematic_directory` 校验非法字符；
  - 已用仓库内 Java 侧的真实文件（`HMCLCore/src/test/resources/schematics/{test.litematic,test.nbt,test.schem,test.schematic}`）
    做读取回归：四种格式都能解析出尺寸/体积/作者/版本等字段；
  - `util/point3.rs`：`Point3I` 提供三维点/尺寸与体积、正尺寸判断。
- **联机目标地址与快捷进入（GAME-07 核心）**：
  - `util/server_address.rs`：`ServerAddress` 复刻 Java `ServerAddress`——`parse` 接受 `主机[:端口]`，
    IPv6 写成 `[地址]` 或 `[地址]:端口`（地址内含多个冒号），拒绝末尾冒号、非数字端口、
    端口越界（> 65535）与括号不完整；`of_host`/`of_host_and_port` 构造，`host()`/`port()`/`has_port()` 读取，
    `to_address_string()` 还原成 `[IPv6]:端口` 形式，`From<&str>` 宽容解析（不合法时只保留主机）；
  - `game/quick_play.rs`：`QuickPlayOption::is_valid()` 对齐 Java `GameSettingsPage` 的校验——
    多人目标必须能被 `ServerAddress` 解析，空白文本视为未填写（合法），单人/Realms 恒合法；
  - `screenshot.rs`：`Screenshot::set_as_background` 把 `background_type` 置为 `Custom` 并写入
    `custom_background_image_path`（对齐 `ScreenshotsPage` 的「设为背景」）。
- **语言标签工具（INF-07 核心，`LocaleUtils`）**：
  - `i18n/locale_utils.rs`：`TextDirection`（`LeftToRight`/`RightToLeft`）、`LanguageTag`
    （语言/书写方式/地区/变体，BCP 47 与 `zh_Hans` 两种写法均可解析，未知标签保留原文）、
    `language_key()`（无语言时取 `default`）、`is_english`/`is_chinese`、`script()`
    （变体 → `UD` 地区 → 默认书写方式表 → 汉语地区（港澳台 `Hant`、其余 `Hans`））、
    `text_direction()`（`Qabs`/`Arab`/`Hebr` 从右到左）、`candidate_locales()`
    （按首选语言 → 父语言链展开，末尾追加 `default`，汉语额外补齐另一种字的地区）；
  - 自由函数：`normalize_language`（空 → `en`，按 `language_aliases.csv` 把 alpha-3 归一到 alpha-2）、
    `parent_language`/`root_language`（`sublanguages.csv`）、`preferred_language`（`zh` → `cmn`）、
    `get_by_candidate_locales`、`find_all_localized_files`（按扩展名）/`find_all_localized_files_by_extension`
    （按多个扩展名，返回「语言键 → 扩展名 → 文件」）；
  - 三张 CSV 数据表从 Java `assets/lang/` 复制到 `crates/hmcl-core/resources/lang/`，
    以 `include_str!` + `OnceLock<BTreeMap>` 载入（与 `version-alias.csv` 一致）；
  - 12 个单测覆盖标签解析、语言键、别名归一、父语言/根语言/首选语言、书写方式推断
    （含 `lzh`→`Hant`、`zh-TW`→`Hant`、`en-UD`→`Qabs`）、文字方向、候选语言顺序
    （`zh-Hans-CN` → `cmn-Hans-CN, cmn-Hans, cmn-CN, cmn, zh-Hans-CN, zh-Hans, zh-CN, zh, default`）、
    候选语言取值、两种本地化文件查找与目录不存在的情形。
- **CSV 表（INF-10，`util/io/CSVTable`）**：
  - `util/csv_table.rs`：`CsvTable` 是「稀疏」表格——`set` 会按需把行、列补空到目标下标，
    行、列计数只记写入过的最大下标 + 1；`get` 对未写入格与越界格都返回空串
    （对齐 Java `InfiniteSizeList` 的 `null` 语义）；
  - `write_to_file` 以 UTF-8 写盘，`Display` 复制 Java 的写出规则：含 `"`/`,`/换行的文本
    加引号且引号翻倍，其余原样写出，每行末尾补 `\n`，整行缺失时写成等列数的空列；
    3 个单测（含 Java `CSVTableTest` 的 `a,b,,c\n,,,\n"a,b",c,,"d""e\n"\n` 逐字对照）
    与文件写回归。
- **光影包（CONT-04 核心；Java 侧无独立管理页）**：
  - `addon/shader_pack.rs`：`ShaderPack::open` 识别 `shaderpacks` 下的目录与 `.zip`/`.jar` 档案，
    `list_shader_packs` 按名字排序，`display_name` 去掉扩展名，`delete` 兼容文件与目录；
  - 启用状态写在 `config/iris.properties` 的 `shaderPack`（对齐 Iris）；`enabled_shader_pack`
    读取、`set_enabled_shader_pack` 写回并保留其他键（临时文件后改名，值未变时不写盘），
    `import_shader_pack` 复制目录/档案且拒绝重名。
- **本地模组与附加内容（CONT-01/CONT-02/CONT-03/CONT-09/CONT-10 核心）**：
  - `addon/local_mod.rs`：`ModFile::open` 读取 `mods` 目录里每个文件的元数据（`.jar`/`.zip`/`.litemod`，
    后缀 `.disabled`/`.old` 依次剥离），依次尝试 Forge（`META-INF/mods.toml`）、NeoForge
    （`META-INF/neoforge.mods.toml`）、Forge 旧版（`mcmod.info`，支持数组与 `modList` 包装）、
    Fabric（`fabric.mod.json`）、Quilt（`quilt.mod.json`，仅 `schema_version == 1`）、
    LiteLoader（`litemod.json`）；元数据读不出时回退到文件名，`has_metadata()` 区分两者；
  - 字段：`id`/`name`/`version`/`game_version`/`authors`/`description`/`url`/`logo`/`loader`；
    Fabric 的 `authors` 支持字符串与 `{ "name": ... }`，Quilt 的 `contributors` 渲染为 `name (role)`，
    Forge 旧版按 `author` → `authors` → `authorList` → `credits` 回退，Forge 新版读取
    `MANIFEST.MF` 的 `Implementation-Version` 填充 `${file.jarVersion}`；
  - `analyze_loader`：Forge/NeoForge 的 `dependencies.<modId>[].modId` 为 `forge`/`neoforge` 时
    覆盖声明的加载器（对齐 Java `ForgeNewModMetadata.analyzeLoader`）；
  - `addon/mod_loader.rs`：`ModLoaderType` 8 种（含 `Unknown`/`Cleanroom`/`LegacyFabric`），
    提供显示名、仓库名、启动器环境变量名；
  - `addon/mod_manager.rs`：`ModManager` 扫描 `mods`（Forge/Quilt/Cleanroom/LiteLoader 同时扫描一级子目录，
    跳过 `.connector`），按文件名校验大小写排序；`disable_mod` 追加 `.disabled`、`enable_mod` 移除、
    `set_old` 追加 `.old`（禁用态先启用）、`delete_mod`、`add_mod`（复制而非移动）、
    `has_simple_mod`（三种状态任一存在）、`rollback`（新旧同名跳过、id/加载器不同拒回滚）；
    `compute_supported_loaders` 对齐 Java：Cleanroom→Forge、NeoForge 1.20.1→Forge、Quilt/Legacy Fabric→Fabric、
    Kilt→Forge+NeoForge、Sinytra Connector→Fabric，无加载器时视为支持全部；
    `loaders_of(&ComponentAnalyzer)` 由实例组件推导游戏加载器；
  - `addon/pack.rs`：`PackManager` 读取 `resourcepacks`/`datapacks`，`Pack::open` 支持目录（需含
    `pack.mcmeta`）与 zip；`PackInfo` 解析 `pack_format`、`supported_formats`（数组或
    `min_inclusive`/`max_inclusive`）、`min_format`/`max_format`（数字或 1/2 元数组，末版本 `minor = MAX`）、
    `description`（字符串或组件拼接）；
  - `PackCompatibility` 对齐 Java `ResourcePackManager.getCompatibility`：所需版本 >64 用新格式、
    >15 用 `supported_formats` 区间、否则只用 `pack_format`；区分
    `Compatible`/`TooNew`/`TooOld`/`Invalid`/`MissingPackMeta`/`MissingGameMeta`；
  - 资源包启用状态读 `options.txt` 的 `resourcePacks`/`incompatibleResourcePacks`，1.13+ 写作
    `file/<文件名>`、更早只写文件名；`enable`/`disable`/`rename`/`is_enabled` 原样保留其他选项，
    写盘走临时文件后改名；数据包不读写选项，改为按文件名启用（目录改 `pack.mcmeta` ↔
    `pack.mcmeta.disabled`、档案改 `<名>.zip[.disabled]`，对齐 Java `DataPack.Pack.setActive`），
    `Pack::is_active()`/`set_active()` 记录并切换该状态，`Pack::open` 也接受已禁用的目录与档案
    （文件名剥离 `.disabled` 后再去扩展名）；
  - `Pack::icon()` 读取 `pack.png`（目录或 zip 根）原始字节，供应用层解码；
  - `pack_version_of` 用已知发布版本表推导所需 `pack_format`（1.6.1→1、1.9→2、1.11→3、1.13→4、
    1.15→5、1.16.2→6、1.17→7、1.18→8、1.19→9、1.19.3→12、1.20→15、1.21→34），
    未列出的新版本按最新已知条目处理；Java 从 `version.json` 读取的逻辑尚未移植；
  - `util/toml.rs`：手写 TOML 读取器（表、表数组、点号键、基本/字面量字符串含多行与转义、
    整数含十六/八/二进制、浮点、布尔、数组、内联表、日期按文本保留、注释与空行），
    供 `mods.toml` 解析使用（离线环境无 TOML crate）；
- **附加内容更新（CONT-08 核心）**：
  - `addon/remote_addon.rs`：`UpdateRestriction` 把 Java `AddonUpdateConditions` 里的谓词换成
    可枚举的数据（目前只有 `Loader(ModLoaderType)`），调用方可以把它展示给玩家；`check_updates`
    在 `find_update` 的基础上先按条件过滤候选版本；`check_update_of_sources` 复刻
    `AddonCheckUpdatesTask` 的「每个仓库各自取最新、两个通道分开选、没有共同候选就没有更新」；
    `updates_of` 保持无条件的旧签名，`updates_of_with_restrictions` 接受每个文件的附加条件；
  - `addon/update.rs`：`AddonKind` 编码 Java 里每个 `LocalAddonFile` 的覆写——
    `Mod { loader }` 对应 `LocalModFile`（按加载器筛选、保留旧文件、按文件名停用），
    `ResourcePackArchive` 对应 `ResourcePackZipFile`（不筛选、更新后删除旧文件、`disabled`
    后缀无意义，因为 Java `ResourcePackFile.markDisabled()` 是空实现）；`AddonKind::of_mod`/
    `of_pack` 与 `AddonUpdateRequest::of_mod`/`of_pack` 负责从 `ModFile`/`Pack` 生成请求，
    数据包与目录形态的资源包返回 `None`（不可更新）；
  - `install_update` 按 `AddonUpdateTask` 的顺序执行：下载到临时文件（校验仓库给的最强摘要）→
    旧文件改名为 `.old`（停用态先剥掉 `.disabled`）→ 新文件改名到位 → 失败时回滚备份并恢复停用态 →
    成功后按 `AddonKind` 决定保留或删除备份；`install_updates` 逐个执行并收集失败原因，
    对应 Java 的 `failedAddons`；
  - 与 Java 的差异：临时文件放在附加内容目录内（`.hmcl-addon-update-<pid>-<n>.tmp`）而不是系统临时
    目录，保证最后一步改名不跨文件系统；回滚用普通文件改名完成，不调用 `ModManager::set_old`，
    因此模组与资源包共用一套逻辑；调用方在更新后需重新读取目录，因为管理器缓存已读的文件；
  - 单测：`update` 8 个（模组保留备份、资源包删除旧文件、停用模组更新后仍停用、下载失败保留原文件
    且不留临时文件、摘要不符拒绝、各类别与 Java 覆写一致、`ModFile`/`Pack` 到请求的映射），
    `remote_addon` 新增 5 个（加载器条件、资源包无条件、跨仓库两通道分开选、无共同候选、带条件批量）。
- **导入（CONT-10 核心）**：
  - `addon/import.rs`：`import_mods`/`import_resource_packs`/`import_data_packs` 分别对应
    `ModListPage.add`、`ResourcePackListPage.addFiles`、`DataPackListPage.installMultiDataPack`；
    每个文件单独导入并记录失败原因，一个文件失败不影响其余（对齐 Java 的 `succeeded`/`failed` 列表）；
  - `ImportReport` 区分三种结果：`imported`（已导入，含落地的目标路径）、`failed`（页面接受但
    导入失败，带原因）、`skipped`（页面不接受，对应 Java 文件选择器与拖拽监听器的过滤条件），
    `is_complete()` 只在没有失败时为真；
  - 过滤条件：模组用 `ModFile::is_file_name_mod`（`.jar`/`.zip`/`.litemod`，含 `.disabled`/`.old`
    状态）、资源包用 `is_file_resource_pack`（含 `pack.mcmeta` 的目录或档案）、数据包只收 `.zip`
    （大小写不敏感，对齐 `DataPackListPage` 的拖拽过滤）；
  - 数据包导入复刻 `DataPack.installPack`：档案里有 `datapacks/` 视为多包合集，解压到世界的
    `datapacks`（不保留 `datapacks/` 这一层）；否则要求根上有 `pack.mcmeta`，按原文件名整包复制；
    两者都缺则报 `NotADataPack`，对应 Java 的 `Malformed datapack zip`；
  - 同名包先删后写：`pack_name_of_file` 先剥 `.disabled` 再去扩展名（对齐
    `FileUtils.getNameWithoutExtension` 与 Java 循环里的 `removeSuffix`），因此启用态、停用态、
    目录态的同名包都会被替换，其他包保留；
  - 合集携带的 `resources.zip` 合并进世界的资源：按版本决定位置（`26.1-snapshot-6` 及以后写到
    `resourcepacks/resources.zip`，更早或版本未知写到世界根），已有内容先读入再覆盖同名条目，
    去掉 `pack.png`，写入固定 `pack.mcmeta`（`pack_format` 4，`"Modified by HMCL."`），
    对齐 Java 用 `ZipFileSystem` 逐条复制 + `ZipWriter` 重写的做法；
  - 与 Java 的差异：Java 用 `FileSystem` 就地增删条目，这里把整个 `resources.zip` 读进内存后
    重写（与 `util/zip.rs` 的既有实现一致）；`PackManager` 仍缓存已读的包，调用方在导入后需要
    重新 `refresh`；
  - 单测 12 个（模组导入与跳过、逐个失败隔离、目录资源包连同文件复制、单包复制、合集解包、
    两种资源布局、资源合并、同名替换（含停用目录）、非数据包档案拒绝、目录被数据包页跳过、
    名字剥离规则）。
- **尚未接入**：模组/资源包/数据包列表页 UI、附加内容更新页 UI（CONT-08 的核心已就绪）、
  拖拽导入与文件选择器（CONT-10 的 UI 部分）。
- 本轮同时补齐的纯逻辑空白：网关地址（INF-10，`ServerAddress`）、CSV 表（INF-10，`CsvTable`）、
  `LocaleUtils`（INF-07），以及本节的操作系统版本识别（INF-01）。
- **操作系统版本识别（INF-01，`util/platform/OSVersion`）**：
  - `util/os_version.rs`：`OsVersion` 为 `Windows(WindowsVersion)` / `Generic(GenericVersion)` 二分。
  Windows 按四个数字字段（major/minor/build/revision）比较，其余系统按
  `VersionNumber` 比较；`of` 按 `OsType` 分流，`version()` 保留原文本（Windows）或返回
  版本号原文（其余），`is_at_least` 不同系统恒为 false（复制 Java 语义）。
  - `WindowsVersion`：常量 `WINDOWS_2000/XP/VISTA/7/8/8_1/10/11`（Windows 11 与 10 靠 build 22_000 区分）。
  `parse` 从文本起始逐段取数字，容忍 `-unknown` 后缀，无数字时为 0.0.0.0；
  `of_numbers` 在 build/revision 为 0 时省略后缀零（`6.1` 而非 `6.1.0.0`）。
  常量无法在 `const fn` 里格式化文本，因此常量的文本字段为空，由
  `version()` 按数字自动补写。`Ord` 为手写（只比四个数字字段），否则派生的
  `Ord` 会把文本字段也比进去＋对 `10.0.26120` 与 `10.0.26120-unknown` 判出不等。
  - `GenericVersion::new` 在 `OsType::Windows` 时 panic（对齐 Java 的 `IllegalArgumentException`）。
  - `util/operating_system.rs` 新增：`OsType::mojang_name`（windows/linux/osx/""）、`parse_name`（复制
  Java `parseOSName`；Java 侧的 `win`/`mac`/`darwin`/`linux` 包含匹配，否则为 `Unknown`）、
  `OsType::system_version`、自由函数 `is_running_under_wine`（检查 `WINEPREFIX`/`WINELOADER`/
  `WINEDLLPATH`）与 `is_windows_7_or_later`。
  - 系统版本读取：Windows 走原生 `advapi32`（`RegOpenKeyExW`/`RegQueryValueExW`）读
  `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` 的 `CurrentBuild`（`REG_SZ`）与 `UBR`（`REG_DWORD`），
  拼成 `10.0.<build>.<ubr>`；Linux 读 `/proc/sys/kernel/osrelease`（只取开头数字与点）。
  本机实验：读出 `10.0.26300.9711`（build 26300、UBR 0x25ef = 9711），Windows 7+ 判定通过。
  - 20 个新单测（os_version 10 + operating_system 10）；`hmcl-core` 合计 1084 个单测全绿，
    `cargo clippy --offline -p hmcl-core --all-targets` 无警告。
  - 尚未移植：`OperatingSystem` 的字符集/代码页/显示版本部分、`Bits`/`Architecture` 的其余帮手、
    `SystemInfo` 的 GPU/CPU 枚举（INF-02 需要操作系统 API，保持 `[~]`）。
- **确认无需单独移植的类**：
  - `GraphicsAPI`（GAME-05 / EXTRA-21 图形后端核心）：`game/graphics.rs` 已完全移植（
    `Default`/`OpenGl`/`Vulkan`、`minecraft_argument()`、`default_for_version()`、26.2 系列的默认回退、
    `is_supported()`），无需新增代码。
  - `SimpleMultimap`：通用 `Map<K, Collection<V>>` 帮手，Java 侧只有一处调用（
    `LaunchManifestNormalizer.uniqueLibraries`），Rust 已在
    `game/launch/libraries.rs::unique_libraries` 里用 `BTreeMap<String, Vec<usize>>` 原生实现，无需单独移植。
  - `SettingsMap`：只有 `HMCL/` 的向导组件类在用，属于被阻塞的 UI 向导工作，无需单独移植。
- **MultiMC 整合包与内置 Java 库（MP-02 / K-02 部分）**：
  - `modpack/multimc.rs` 的 `MultiMcManifest` / `MultiMcComponent` / `MultiMcRequirement`
    对应 `mmc-pack.json`，`MultiMcConfig` 对 `instance.cfg` 复刻 Java 的 `Properties` 读取
    （`#`/`!` 注释、`=`/`:`/空白三种分隔、`\` 续行、`\t\n\r\f\uXXXX` 转义、无值的键），
    并保留 Java 的 `readValue` workaround（以 `"` 开头且以 `:` 结尾的值去掉末尾冒号）；
    `overrides_memory`/`overrides_java_location`/`overrides_java_args`/`overrides_commands`/
    `overrides_window` 对应 MultiMC 的 `Override*` 开关；
  - `MultiMcInstancePatch` 对应 `patches/<uid>.json`：`formatVersion`（只接受 1）、`uid`、`version`、
    `assetIndex`、`minecraftArguments`、`+jvmArgs`、`mainClass`、`compatibleJavaMajors`、`mainJar`、
    `+traits`、`+tweakers`、`+libraries` 与旧写法 `libraries`（两者都读，`+` 在前）、`mavenFiles`、
    `jarMods`、`requires`；`MultiMcInstancePatch::read` 把 JSON 错误包装成 `MultiMcError::Patch`；
  - `resolve_patches` 对齐 Java `MultiMCInstancePatch.resolveArtifact`：最后一个组件命名标量字段，
    之前的组件按逆序补齐未命名的标量并前置合并列表（库、tweaker、trait、maven 文件、jar mod 文件名）；
    默认主类 `net.minecraft.client.Minecraft`；`--tweakClass` 从 `minecraftArguments` 中抽出并去重后
    统一追加到游戏参数末尾；`FirstThreadOnMacOS` 追加仅在 macOS 生效的 `-XstartOnFirstThread`，
    `XR:Initial`/`texturepacks`/`no-texturepacks` 忽略，其余 trait 记入 `warnings`；
    ForgeWrapper 存在时补 `-Dforgewrapper.librariesDir`/`-Dforgewrapper.minecraft`/
    `-Dforgewrapper.installer=<Forge|NeoForge installer 路径>`；`compatibleJavaMajors` 取排序后
    启动器认识的最大版本写进 `javaVersion`，否则记入 `warnings`；`mainJar` 的下载信息写进
    `downloads.client`；`jarMods` 文件名去重后由应用层叠加到客户端 jar；
  - 内置引导器注入：`bootstrap_library()`（`org.jackhuang.hmcl:mmc-bootstrap:1.0`）插到库列表首位，
    JVM 参数追加 `-Dhmcl.mmc.bootstrap=<profile>`，主类改为 `org.jackhuang.hmcl.HMCLMultiMCBootstrap`；
    `bootstrap_profile_url` 按 `application/x-www-form-urlencoded` 编码
    （空格写 `+`，`.-*_` 与字母数字不转义）拼出
    `hmcl:///bootstrap_profile_v1/?main_class=…&installer=…`，与 `HMCLMultiMCBootstrap` 的查询解析一致；
    `installer_profile()` 写 `Patches` 与 `Implementation` 两行；
  - `component_of_uid` 复刻 `MultiMCComponents` 的 6 条 uid ↔ `GameComponentType` 映射；
    `meta_url` 复刻 `getMetaURL`（缺版本时 `org.lwjgl`→2.9.1、`org.lwjgl3`→3.1.2、
    `net.fabricmc.intermediary`/`org.quiltmc.hashed`→游戏版本，其余返回 `None`）；
    `root_directory` 复刻 `MultiMCModpackProvider.getRootEntryName`（`instance.cfg` 在 zip 根或一级目录下）；
  - 内置 jar 的内容（`HMCLMultiMCBootstrap-1.0.jar`、`HMCLTransformerDiscoveryService-1.0.jar`）
    需由 Gradle 构建 `minecraft/libraries/*` 产出，仓库内没有现成字节，因此注入逻辑已完成、
    字节由应用层经 `read_resource` 提供（与 `repair_libraries` 的约定一致）；
  - 尚未实现：jar mod 叠加。
- **整合包清单、安装与更新（MP-02 / MP-03 / MP-06）**：
  - `modpack/configuration.rs`：`ModpackConfiguration<M>` 对应 `modpack-configuration.json`
    （`type` 由 `#[serde(rename = "type")]` 落到 `type` 字段名、`overrides` 记录覆盖目录，
    `FileInformation` 保留 `path`/`hash`/`downloadURL`）；`read`/`write`/`validate` 把未知类型与
    空清单报成 `ConfigurationError`，读写走临时文件后改名；
  - `modpack/plan.rs`：`UpdatePlan`（`downloads`/`removals`/`warnings`）与 `DownloadRequest`
    （`url`/`path`/`sha1`）是三种格式共用的更新结果；`relative_path` 拒绝绝对路径、盘符与 `..`，
    越界只记 warning 不写盘；`sha1_of` 在读不到文件时返回 `None`（于是该文件会被重新下载）；
  - `modpack/mcbbs.rs`：`McbbsModpackManifest` 对应 `mcbbs.packmeta`（或 `manifest.json`），
    `McbbsFile` 用 `#[serde(tag = "type")]` 读 `addon`/`curse` 两种条目；`read_from_archive`
    先找 `mcbbs.packmeta` 再退到 `manifest.json`；`plan_update` 复刻 Java 的判定顺序
    （文件缺失 → 下载；玩家改过且新条目非 `force` → 保留；`fileApi` 缺失 → 不下载不删除），
    `merge_file` 复用旧清单里已解析的 `fileName`/`url`，未解析的 Curse 条目只报告不下发下载请求
    （`completion::unresolved_files`）；Addon 的下载目标用实例 run 目录，有意修正 Java
    `getSimpleModPath` 写 mods 目录的行为差异；
  - `modpack/server.rs`：`ServerModpackManifest` 读 `server-manifest.json`（`fileApi`/`files`/`addons`），
    组件比较用 `components`/`components_changed`；`is_disabled_mod` 认定同目录 `.disabled`/`.old`
    文件为玩家禁用并跳过；`plan_update` 与 MCBBS 共用同一套「玩家改过就保留」判定；
  - `modpack/install.rs`：`archive_overrides` 计算覆盖文件的 SHA-1，`install` 按 `overrides` 前缀解包、
    保留玩家修改与删除、删掉新版本丢弃的文件（`filter` 回调可选文件）；`update_with_backup`
    先把实例整体复制到 `backup/<id>-<number>`，失败时删半更新目录再复制回滚，成功保留备份；
  - `modpack/completion.rs`：`CompletionReport` 记录阶段（`hmcl.modpack`/`hmcl.modpack.download`）、
    进度与未解析文件，`UnresolvedKind`（`Deleted`/`Unreachable`/`Malformed`）对应 Java
    `ModpackCompletionException` 的三条失败路径，`summary()` 生成「已安装但缺少 N 个文件」的报告文本；
  - `modpack/curse.rs`：`CurseManifest` 读 `manifest.json`（`manifestType` 必须为
    `minecraftModpack`、`minecraft.version` 必填、每个 `files` 条目必须有 `projectID`/`fileID`），
    `CurseFile::url()` 先取清单里的 `url`，缺失时按 `edge.forgecdn.net/files/<id/1000>/<id%1000>/<名>`
    推导（文件名未知则无地址），`integrity_check()` 按 SHA-1 → SHA-256 → SHA-512 → MD5 取第一个非空摘要，
    `key()` 为 `curseforge:<project>:<file>`，`optional()` 取 `!required`；
  - `modpack/modrinth.rs`：`ModrinthManifest` 读 `modrinth.index.json`（`game` 只接受 `minecraft`、
    `dependencies.minecraft` 必填），`ModrinthFile::validate` 拒绝空路径、越界路径、非 SHA-512 摘要、
    缺失 `env.client` 与空 `downloads`；`selected_files(excluded)` 按 `key()` 过滤玩家未选的可选文件，
    `integrity_check()` 优先 SHA-512、其次 SHA-1；
  - `modpack/adviser.rs`：`MODPACK_BLACK_LIST`/`MODPACK_SUGGESTED_BLACK_LIST` 逐条复刻 Java
    `ModAdviser`，`Matcher` 把 `regex:` 前缀编译为**整串匹配**（Java `String.matches` 语义，
    Rust 侧加 `^(?:…)$` 锚点），目录只按名字前缀匹配（与 Java 一致，正则条目不影响目录），
    `suggest_mod()` 返回 `Suggested`/`Normal`/`Hidden`；`accept_file()` 复刻 `Modpack.acceptFile`
    并额外拒绝落在被隐藏目录下的路径，供不剪枝的遍历使用；
  - `modpack/export.rs`：`ModpackExportInfo` 对应 `ModpackExportInfo`（whitelist/url/fileApi/
    minMemory/supportedJavaVersions/launchArguments/javaArguments/authlibInjectorServer/origins/
    forceUpdate/packWithLauncher/noCreateRemoteFiles/skipCurseForgeRemoteFiles），`ExportOptions`
    的 `MCBBS`/`SERVER`/`MODRINTH` 三套必填项对应 Java 各导出任务的 `OPTION`；`collect_files` 按名字
    排序遍历 run 目录、跳过 `.DS_Store`、套用黑名单与白名单并算 SHA-1；`export_addons` 按
    `ComponentAnalyzer` 的 `Mark` 生成 `game` + 加载器 + OptiFine 组件；`write_mcbbs` 写
    `overrides/` + `mcbbs.packmeta` + CurseForge 兼容的 `manifest.json`，`write_server` 写
    `overrides/` + `server-manifest.json`，`write_modrinth` 写 `client-overrides/` +
    `modrinth.index.json`（能对上仓库的 mod 交给调用方的 `remote` 回调进清单，其余进覆盖目录）；
  - 尚未实现：CurseForge/Modrinth 的远程文件名与项目查询（需要 HTTP）、MCBBS 更新时的在线清单拉取、
    整合包安装向导 UI 与 `pack_with_launcher` 的启动器打包。
  - `modpack/curse.rs`/`modrinth.rs`：`plan_completion`（ excluded 跳过、env/excluded/exists 跳过、
    has_simple_mod 跳过、`class_id`→`mods`/`resourcepacks`/`shaderpacks` 目标目录、`relative_path` 校验、
    `removed_mods`/`removed_files` 删除旧版本独有的 mod）；`completion.rs` 的 `EXCLUDED_FILE` 读写与
    `has_simple_mod`；`CompletionPlan`/`CompletionDownload` 统一下载描述。
  - `completion.rs`：`report_of_curse`/`report_of_modrinth`、`UnresolvedFile::modrinth`、
    `UnresolvedKind::NotResolved` 与 `CompletionError`，外加 2 个报告测试。
  - MP-03 更新（版本差异/文件替换）：`CompletionPlan::removals` 承载更新删除；
    `plan_completion_with_updates`（Curse 按 project+file id 比较并跳过 `overrides` 的 mod、
    Modrinth 按 path/hashes/env/downloads/fileSize 全等比较）；
    `install::update_with_backup` 提供「先备份→更新→失败回滚」的 `ModpackUpdateTask` 语义。
  - 尚未实现：CurseForge/Modrinth 的远程文件名与项目查询（需要 HTTP）、MCBBS 更新时的在线清单拉取、
    整合包安装向导 UI 与 `pack_with_launcher` 的启动器打包。
- **设置迁移（SET-08，`LegacyConfigMigrator` / `LegacyGameSettingsMigrator`）**：
  - `setting/legacy_migration.rs` 读旧版 `config.json`（`_version` 高于当前旧版格式则整份忽略），
    把旧字段改名/换算为当前 `LauncherSettings`：`commonDirType`→`commonDirectoryType`、
    `commonpath`→`commonDirectory`、`localization`→`language`、`theme`→`customThemeColor`（并置
    `themeColorType = CUSTOM`）、`themeBrightness`/`titleTransparent`/`windowTransparent`/
    `bgImageOpacity`（百分比 → 0–1）等同时记入 `themeAppearanceOverrides`（已被覆盖的不再改写）、
    `proxyType`（Java 序号 DIRECT/HTTP/SOCKS）与 `proxyUserName`→`proxyUser`、`downloadType` 的
    多候选源合并、背景类型（DEFAULT/CUSTOM/BUILTIN/NETWORK/TRANSLUCENT/PAINT）与回退/加载策略；
  - 升级 HMCL 2.x 的老配置：离线账户、`commonDir`、背景图、代理、下载源等旧写法；
  - 拆出到独立文件的部件放在 `DetachedSettings`：窗口状态（`x`/`y`/旧版外框尺寸换算成内容尺寸、
    `promptedVersion`、`shownTips`）、authlib-injector 服务器列表、游戏目录列表、游戏设置预设；
  - 记录 ID 用 UUID v5（Java 的三个命名空间）：profile `d5193464-…`、preset `4ba56ddd-…`、
    account `cdc608bb-…`，分别写作 `game-directory:<uuid>`、`game-settings-preset:<uuid>`、
    `account-<uuid>`；同一份旧配置反复迁移得到同一批 ID；
  - 迁移回执记 `sourceSize`/`sourceSHA256`/`migratedAt`，落在 `state/launcher-settings.migration-receipt.json`，
    文件内容未变则不再迁移；迁移只写当前启动器尚未写过的文件，玩家的现有设置不会被覆盖；
  - `setting/legacy_game_settings.rs`：旧 profile 的 `global` 对象 → `Preset`（`gameDirType = VERSION_FOLDER`
    落成 `DefaultIsolationType::Always`），`allowAutoAgent`/`disableAutoGameOptions` 写进所有预设
    （一个预设都没有时新建默认预设）；旧 `hmclversion.cfg` → `InstanceSettings`（`usesGlobal` 决定继承还是
    覆盖，覆盖时把旧版认识的全部 38 个字段标记为已覆盖，图标/Java 选择/渲染器/图形后端/快捷进入/内存/
    优先级/可见性等按 Java 的序号与名称两种写法解析）；
  - `migrate_legacy_instance_settings` 在实例已有 `instance-game-settings.json` 或回执匹配时返回 `None`；
    `migrate_legacy_instances` 遍历 `<游戏目录>/versions/*`，只迁移带 `<名称>.json` 的目录；
  - `Launcher::load()` 在读设置前调用 `migrate_legacy_settings`，失败只记警告不阻断启动；
  - 账户迁移（`AccountMigrationResult`）：`extract_accounts` 取出 `accounts` 列表并处理 `selectedAccount`
    与记录里的 `selected` 标记；`migrate_legacy_accounts` 把 offline/microsoft/yggdrasil/authlibInjector
    四类记录拆成公开元数据与凭据（凭据写入 `AccountPrivateData`，未知类型原样保留）；
    账户 ID 用 UUID v5 派生（`name:name`、`microsoft:<uuid>`、`loginName:<uuid>`、
    `<serverBaseURL>:<loginName>:<uuid>`，共享文件加 `$GLOBAL:` 前缀），已用 Java oracle 校验；
    `assign_legacy_account_ids` 处理跨文件重名；`migrate_legacy_settings` 在 `config/accounts.json`
    不存在时写入元数据与私有数据；
  - 共享账号：`migrate_legacy_user_accounts` 读 `<user_home>/accounts.json`（旧版跨工作区账号列表），
    以 `$GLOBAL:` 前缀派生 ID 后写入 `config/user-accounts.json` 与
    `private/user-account-private-data.json`，回执写 `state/user-accounts.migration-receipt.json`；
    `AccountMetadataStore`/`AccountPrivateData` 增加 `load_from`/`save_to` 以复用同一份格式；
    `Launcher::load()` 在设置迁移之后调用；
  - 尚未实现：把游戏目录与实例设置写盘（`DetachedSettings` 的游戏目录与预设由调用方写，
    实例设置需游戏目录就绪后再迁移）。
- 说明：离线环境未缓存 `thiserror`/`tracing`/`tokio`，错误类型与日志为手写实现，异步运行时待后续引入。
- 构建 `hmcl` 应用 crate 需要三个环境变量（网络不可用时用本地缓存的 Skia 预编译包）：
  - `LIBCLANG_PATH` → `D:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Tools\Llvm\x64\bin`
    （skia-bindings 的 bindgen 需要 libclang）；
  - `LLVM_HOME` → 同上的上一级目录（skia-bindings 的构建脚本需要 clang）；
  - `SKIA_BINARIES_URL` → `file://C:/.../skia-binaries.tar.gz`（构建脚本支持 `file://`，
    指向之前下载的 `skia-binaries-<key>.tar.gz`，否则它会去 GitHub 下载）。
- 应用层（`hmcl-rs/crates/hmcl`）：已接入核心——启动时读取设置（带 schema 兼容检查）、按用户语言初始化 i18n、
  在 `logs` 目录创建日志文件、窗口文本由 `x:Name` 定位后填入本地化文本；
  `resources.rs` 建立应用级资源（主题配色 + 全部本地化文本），转换来的页面直接引用。已实际启动验证（日志、设置、
  XAML 窗口均正常）。
- 目录约定与 Java 对齐：`metadata.rs` 的 `user_home`/`local_home` 与 Java `Metadata.HMCL_USER_HOME`/
  `HMCL_LOCAL_HOME` 一致（Windows `%APPDATA%\.hmcl`、Linux `$XDG_DATA_HOME/hmcl`、本地目录 `<cwd>/.hmcl`），
  Rust 启动器可读写 Java 启动器的 `config/user-settings.json`。
- **高级启动选项（EXTRA-24，`GameSettingsPage` 高级）**：
  - `game/launch/command.rs`：`LaunchOptions` 新增 `no_generated_optimizing_jvm_arguments`（对应
    `noOptimizingJVMOptions`）与 `java_bits`，`build_command` 按 Java `CommandBuilder` 的顺序重排：
    内存参数 → 玩家 `jvm_arguments` → 默认 JVM 参数（编码/log4j/RMI/JNDI + 优化块）→
    清单 JVM 参数 → `-javaagent:`；优化块按运行时位数启用 G1 相关 `-XX:` 参数，
    并按 Java 版本追加 `--illegal-access=permit`（16）、`--sun-misc-unsafe-memory-access=allow`（24–25）
    与两个 `-Dfml.*` 默认值；`need_patch_memory_util` 对应 `NativePatcher.needPatchMemoryUtil`；
  - `game/launch/settings.rs`：`apply_settings` 从 `java.platform()` 读位数与平台（`java_bits`/
    `java_platform`，native 替换用后者而非本机平台）；`find_launch_java` 复刻
    `checkGameState` 的 `notCheckJVM` 短路：设置挑不出运行时且玩家关掉检查时，
    用 `java::default_java_runtime()`（PATH 上的 Java）兜底；能挑出合适运行时时仍优先用它；
  - `game/launch/native_patch.rs`：`patch_natives` 复刻 `NativePatcher.patchNative` 的六步——
    `useCustomNatives` 丢弃 LWJGL natives（游戏 ≥ 1.19）、Linux/BSD 上 `useNativeGLFW`/`useNativeOpenAL`
    过滤 glfw/sdl/openal 的 natives、`notPatchNatives` 直接返回、x86 与 macOS/Windows arm64（1.19+）
    跳过，最后按 `resources/game/natives.json` 做库替换（值为 `null` 表示删除该库；
    键为 `OperatingSystem.getCheckedName() + "-" + Architecture.getCheckedName()`，如 `windows-x86_64`）；
    补丁后的清单必须在 `checkGameCompletionAsync`/`getLaunchOptions` 之前生效，因此
    `plan_launch` 先从客户端 jar 取 `minecraft_version`（读不出回落
    `UNKNOWN_GAME_VERSION = "0.0"`，对应 Java `GameVersionNumber.unknown()`）再打补丁，
    并把补丁产生的 native JVM 参数拼进 `jvm_arguments`；
  - 与 Java 的差异：Java `patchNative` 用 `"org.lwjgl:lwjgl".equals(library.name())` 判定，而
    `name()` 返回含版本的完整描述，永远匹配不上；Rust 改为判断 `group`/`name`/
    无 classifier，且仅在 `!native` 时生效（Sodium 分支）；Rust `Arch` 无 riscv64/
    loongarch64/mips64el，`linux-riscv64` 等键实际不可达；
  - 单测：`native_patch` 13 个，`settings` 的 `notCheckJVM`/平台 3 个，`java::discovery` 的
    `default_java_runtime` 1 个；`plan_launch` 1 个——客户端 jar 写入 `version.json` 后
    按 `linux-arm64` 运行时打补丁，验证 `required_files` 换成 3.3.2 的 natives、
    且 `-Dsodium.checks.issue2561=false` 进入 `jvm_arguments`。
- 待办：图形硬件信息（GPU）与 CPU/GPU 枚举（INF-02，需要操作系统 API）、IO 工具、下载与实例等后续阶段。

### 阶段 R1：认证（2 周）
- 离线 UUID 派生、Microsoft OAuth/XBL/XSTS/Minecraft、Yggdrasil、authlib-injector（jar 下载/预取/参数）。
- 令牌刷新与持久化（`keyring` 可选）。
- 验收：三类账户可登录并生成启动参数。

**进度（Rust）**
- 已完成（AUTH-02 的核心链路）：
  - `util/http.rs`：`HttpMethod`/`HttpRequest`/`HttpResponse`/`HttpClient`/`RequestFailure`/`send`/`send_json`/
    `encode_form_component`/`form_body`/`with_query`，重定向与 Java `NetworkUtils.resolveConnection` 一致
    （手动跟随 `Location`、最多 20 跳、拒绝非 HTTP 协议、未编码的 location 做百分号编码），
    请求头取 `default_headers(language)`；
  - `util/random.rs`：`bytes::<N>()` 用计数器 + 时钟 + 进程号的 splitmix64 生成随机字节，注释里写明
    **不是密码学安全**的来源（离线环境没有缓存的 `rand`/`getrandom`）；
  - `util/base64.rs`：`encode_url_safe_unpadded`（对应 `Base64.getUrlEncoder().withoutPadding()`），
    PKCE 的 verifier 与 state 都用它编码；
  - `auth/error.rs`：`AuthenticationError` 为数据枚举（20 个变体，含 `LoginAborted`、`ServerDisconnect`、
    `ResponseCode`、`RemoteAuthentication`、`XboxAuthorization`、`NoUserHash`、`XboxBadRequest`、
    `MicrosoftNotSupported`），`From<RequestFailure>` 把网络失败映射为 `ServerDisconnect`、把非 2xx
    映射为 `ResponseCode`，`requires_login()` 区分「可以让玩家重试」与「让玩家重新登录」；
  - `auth/oauth.rs`：`OAuth`（`OAuth::microsoft()` 的端点常量）、`GrantFlow`（`AUTHORIZATION_CODE`/`DEVICE`）、
    `OAuthCallback`/`OAuthSession` 契约、授权码流与设备码流（PKCE S256、state 校验、`retry(5)`、
    900 秒设备码上限由私有 `Clock` trait 注入，测试用 `TestClock` 而不真正睡眠）、`refresh`/`validate`；
    刷新应答没有新 refresh token 时保留旧的那个（`RefreshResponse::into_tokens(previous)`）；
  - `auth/microsoft.rs`：`authenticate`/`refresh`/`validate`/`get_minecraft_profile`，
    OAuth → XBL (`user.auth.xboxlive.com`) → XSTS (`xsts.auth.xboxlive.com`) → Minecraft
    (`api.minecraftservices.com/authentication/login_with_xbox` → `/minecraft/profile`) 的完整链，
    `get_uhs` 校验两次应答的 user hash 一致，`MinecraftLogin`/`MicrosoftSession`/`MicrosoftLogin`
    对应 Java 的 `MicrosoftAccount.logIn`/`logInWhenCredentialsExpired`/`playOffline`/`writeMetadata`/`writePrivateData`；
  - `crates/hmcl/src/http_client.rs`：`UreqHttpClient` 实现 `HttpClient`，复用
    `http_fetcher::build_agent` 与 `http::default_headers`，手动重定向时保留 `Location`；
  - `crates/hmcl/src/oauth_server.rs`：`LocalServer` 在 `127.0.0.1` 的 29111–29115 上监听 `/auth-response`，
    `Page` 渲染内嵌的 `microsoft_auth.html`（`%lang%`/`%style%`/`%success%`/`%ok%`/`%close_page%`，
    颜色取自窗口画笔而不是 `Themes.toColorScheme()`，因为 Rust 侧没有 Monet 实现），
    `DesktopOAuthCallback` 打开浏览器（Windows `rundll32 url.dll,FileProtocolHandler`、macOS `open`、
    Linux 依次试 `LINUX_BROWSERS`），客户端 ID 取 `HMCL_MICROSOFT_AUTH_ID` 环境变量或编译期同名变量，
    为空时在占用端口之前就以 `MicrosoftNotSupported` 拒绝登录；
- 与 Java 的差异（均有注释与测试说明）：
  - Java 的 `Factory` 触发事件并保留静态 `lastlyOpenedURL`；Rust 只保留 `last_opened_url` 字段，
    设备码写进日志，因为事件总线与窗口尚未接上；
  - `send` 遇到非 2xx 立即返回（不重试状态码），只有连接失败才重试，避免把服务器的 4xx 重复放大；
  - `authenticate_device` 会通过 `check_error` 检查服务器拒绝，Java 侧直接返回令牌；
  - `OAuthSession` 没有 `id_token` 访问器（Java 的 `idToken` 未被使用）；
  - `MicrosoftService.getXboxProfile` 与纹理/档案属性缓存未移植；
  - XBL/XSTS 拒绝应答里的 `Message` 与 OAuth 拒绝应答里的 `correlation_id` 不再反序列化：
    Java 侧同样只读 `XErr`/`Redirect` 与 `error`/`error_description`，这两个字段只是原样留着的副本；
  - 请求逐行读取时，超长行（`MAX_LINE_BYTES`）直接回 400 而不是把余下部分当成下一行头部，
    被拒绝的请求会先读完剩余字节再关闭连接，避免 Windows 发出 RST 让浏览器丢掉应答；
- 单测：`util/http.rs` 18 个、`util/random.rs` 2 个、`util/http_request.rs` 13 个、`auth/error.rs` 5 个、
  `auth/oauth.rs` 16 个、`auth/microsoft.rs` 25 个、`http_client.rs` 5 个、`oauth_server.rs` 25 个；
  （该阶段完成时 `hmcl-core` 1245 / `hmcl` 46 个单测全绿；当前合计见本节末尾）
- 已完成（AUTH-03 的核心链路）：
  - `auth/yggdrasil/provider.rs`：`YggdrasilProvider` trait 与 `AuthlibInjectorProvider`，
    `authserver/authenticate`、`authserver/refresh`、`authserver/validate`、`authserver/invalidate`、
    `api/user/profile/<紧凑 uuid>/skin`、`sessionserver/session/minecraft/profile/<紧凑 uuid>`；
  - `auth/yggdrasil/model.rs`：`GameProfile`/`CompleteGameProfile`/`User`/`Texture`/`TextureType`/
    `TextureModel`，properties 在服务器侧是 `[{name,value}]` 列表、在本地存储里是 `{name: value}` 对象，
    `CompleteGameProfile::textures()` 解码后只保留 `SKIN`/`CAPE`；
  - `auth/yggdrasil/service.rs`：`YggdrasilService`（authenticate/refresh/validate/invalidate/upload_skin/
    complete_game_profile），每请求 `.ignore_status().attempts(5)`，档案缓存
    `Mutex<BTreeMap<Uuid, CompleteGameProfile>>`，应答的 `clientToken` 与请求不一致即
    `"Client token changed from ... to ..."`；
  - `util/multipart.rs`：`MultipartForm`，boundary 为 `*****<millis>*****`，与 Java 的
    `HttpMultipartRequest` 逐字节一致（未引入 `reqwest`/multipart 依赖）；
  - `auth/yggdrasil/session.rs`：`YggdrasilSession` 的存储读写与 `to_auth_info`；
  - `auth/yggdrasil/account.rs`：`YggdrasilAccount` 镜像 Java 的 `logIn`/`logInWithPassword`/
    `playOffline`/`writeMetadata`/`writePrivateData`，`CharacterSelector` 由调用方提供页面，
    客户端令牌用 `util::random` 生成 32 位十六进制；
- 与 Java 的差异（AUTH-03）：`CharacterSelector` 是 trait（而不是静态方法），`uploadableTextures`
  读取大小写不敏感但不使用 `Collections.unmodifiableSet`；其余行为与 Java 对齐；
- 已完成（AUTH-04 的核心链路）：
  - `crates/hmcl-core/resources/assets/authlib-injector-1.2.8.jar`：随二进制编入的 authlib-injector
    （`include_bytes!`，349681 字节），版本常量 `AUTHLIB_INJECTOR_VERSION = "1.2.8"`；
  - `metadata.rs`：`DEPENDENCIES_DIRECTORY_VARIABLE = "HMCL_DEPENDENCIES_DIR"` 与
    `dependencies_directory()`（回退 `local_home()/dependencies`，结果绝对化），对应
    `Metadata.getDependenciesDirectory()`；
  - `auth/authlibinjector/artifact.rs`：`AuthlibInjectorArtifactInfo::from_path` 读 manifest 的
    `Implementation-Title`/`Implementation-Version`/`Build-Number`（错误文案与 Java 逐字一致），
    `AuthlibInjectorExtractor` 双检锁提取（同目录 `.<文件名>.tmp` 再 rename，等价 `FileUtils.saveSafely`），
    `SimpleAuthlibInjectorArtifactProvider` 与 `AuthlibInjectorArtifactProvider` trait；
  - `auth/authlibinjector/account.rs`：`AuthlibInjectorAccount`（运行期账号，与 `account.rs` 里的
    序列化记录 `AuthlibInjectorAccount` 同名但分离），`authenticate`/`new`/`from_storage`/
    `log_in`/`log_in_with_password`/`play_offline`/`write_metadata`/`write_private_data`/
    `clear_cache`/`uploadable_textures`，登录时先取 artifact 再取 metadata，失败分别映射为
    `InjectorDownload` 与 `ServerDisconnect`；`launch_info` 生成三条 JVM 参数
    （`-javaagent:<jar>=<server>`、`-Dauthlibinjector.side=client`、
    `-Dauthlibinjector.yggdrasil.prefetched=<Base64(metadata 原文)>`）；
  - `auth/auth_info.rs`：`AuthInfo.launch_arguments`（`#[serde(skip)]`，对应
    `AuthInfo.getLaunchArguments` 的 `null` 语义）与 `with_launch_arguments`；
  - `game/launch/command.rs`：参数顺序对齐 `DefaultLauncher.generateCommandLine`——manifest JVM 参数 →
    `auth.launch_arguments` → `options.java_agents` 的 `-javaagent:` → main class；游戏侧
    quickPlay → `--fullscreen` → `--graphicsBackend <default|opengl|vulkan>`（仅
    `graphics_api != DEFAULT` 且游戏版本 `>= 26.2-snapshot-2`，与 `GraphicsAPI.getMinecraftArg()` 一致）→
    玩家参数，最后按 Java 的 `forbiddens` 在 Java ≥ 9 时移除 `-Xincgc`；`build_command` 因此新增
    `game_version: &GameVersionNumber` 参数，`plan_launch` 传入已算出的版本；
- 与 Java 的差异（AUTH-04）：`AuthlibInjectorAccount` 不派生 `Debug`（`Arc<dyn ...Provider>` 无 `Debug`），
  改为手写 `debug_struct`；`set_metadata_response` 只保留 `serverName`/`links`/
  `feature.non_email_login` 三个字段的解析（Java 把整段 JSON 存为 `JsonObject`，Rust 存原始文本再按需取值）；
  `AuthlibInjectorServer` 在 Rust 侧只是设置值，运行期账号另持
  `Arc<YggdrasilService<AuthlibInjectorProvider>>`（`service_of` 每次新建），因此
  `invalidate_metadata_cache` 与档案缓存不在同一个对象里；
- 单测：`auth/authlibinjector` 27 个（artifact 15、account 14，其中 2 个跨模块）、`command.rs` 21 个
  （新增 graphics backend 门控 3 个、账户参数顺序 1 个、java agents 1 个、`-Xincgc` 2 个）；
- 已完成（AUTH-05 的运行时装配）：
  - `setting/authlib_injectors.rs`：`AuthlibInjectorServerMetadataCache` 与
    `AuthlibInjectorServerList` 的读写（`authlib_injector_servers_path` 解析出
    `<user_home>/config/authlib-injectors.json`），失败只记日志并回退到空列表；
  - `setting/mod.rs`：`is_new_workspace()` 在**任何文件写入之前**求值，因为 Java 的
    `isNewWorkspace()` 把 `hasLegacyConfig()` 也算作「已有工作区」；
  - `auth/authlibinjector/factory.rs`：`AuthlibInjectorAccountFactory` 按 URL 定位服务器
    （`locate_server` 在后台线程里逐个 URL 执行，失败只记日志不加入列表），
    `add_server`/`get_or_add` 之后元数据缓存同步落盘；
  - `crates/hmcl/src/context.rs`：`Launcher` 持有 `authlib_servers`/`authlib_metadata`/
    `newly_created`/`pending_authlib_servers`，`import_configured_authlib_servers()` 负责
    启动时回填，`add_located_authlib_server()` 负责把定位到的服务器写进列表与缓存；
  - `authlib-injectors.json` 的查找路径：可执行文件同目录（是常规文件且可写）优先，
    否则当前目录，用 `std::env::current_exe()`；
- 与 Java 的差异（AUTH-05）：
  - Rust 侧没有事件总线，元数据缓存由装配层显式调用（启动回填 + 增删时读写），
    Java 侧是 `SettingsManager` 的事件监听；
  - Java 把 `config`/`state`/`cache` 分到 `HMCL_LOCAL_HOME` 的不同子目录，Rust 侧统一放在
    `user_home` 之下（刻意的偏差，已在多处注释说明）；
  - `preferred_login_type` 只在 `load()` 内通过 `launcher_settings.settings_mut()` 直接改；
- 单测（AUTH-05）：`setting/authlib_injectors.rs` 6 个、`setting/mod.rs` 3 个、
  `AuthlibInjectorServerList` 27 个、`auth/authlibinjector/factory.rs` 11 个；
  当前 `hmcl-core` 1290 个单测、`hmcl` 45 个单测全绿，`cargo fmt --all --check` 通过，
  `cargo clippy --offline --all-targets` 在两个 crate 上均无警告；
- 尚未接入：账户页与登录向导（UI-01）、令牌持久化到账户私有数据（`keyring` 可选）、
  多角色选择页（AUTH-07）、皮肤/披风上传页（AUTH-09）、`OfflineAccount` 的 localhost agent。

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
- UI 技术已定为 FerroUI（FerroUI 是 Avalonia 的 Rust 移植），按第 2 节布局实现：
  主框架/侧边栏 → 启动页/日志窗口 → 账户页 → 实例列表/详情 → 下载分类/版本/附加内容 → 设置分类页 → 向导/对话框。
- 界面设计复用 C# 分支的 Avalonia XAML（FerroUI 的标记语法、控件集、样式系统与 Avalonia 一致）：
  - `hmcl-rs/tools/convert_avalonia_xaml.py` 把 `HMCL.NET` 分支的 22 个 XAML 机械转换成 FerroUI 版本
    （换命名空间、删 `x:Class`/`d:Design*`/`mc:Ignorable`、`clr-namespace:` 改标记命名空间
    `HMCL.Views`/`HMCL.ViewModels`/`HMCL.Controls`/`HMCL.Util`），产物在 `hmcl-rs/crates/hmcl/ui/pages/`；
  - 已验证：转换后的页面能被 FerroUI 运行时加载器完整解析——`TaskListPage` 连同 Rust 侧的视图模型
    （`TaskListViewModel`/`TaskItemViewModel`，`ferro_class!` + `ferro_property!` + `ferro_class_info!`）
    已能加载，`cargo test -p hmcl` 有 3 个用例守着这条链路；
  - 类型注册：`crates/hmcl/src/types.rs` 把 Rust 模块路径映射到标记命名空间（`HMCL.ViewModels` 等），
    并注册同名程序集，运行时加载器才能按 `clr-namespace:` 找到类型；
  - 已 Rust 化：视图模型（`TaskListViewModel`/`TaskItemViewModel`/`MainWindowViewModel`）、
    展示模型（`BlueprintInfo`/`DataPack`/`ResourcePack`/`GameComponentMark`/`ScreenshotInfo`/
    `WorldInfo`/`GameInstance`）、转换器（`BoolToTextConverter`/`NullToBoolConverter`/
    `NotNullToBoolConverter`/`VersionTypeConverter`）；`display_model!` 宏一键生成可绑定的类；
  - 已跑通可加载的页面（9 个，`cargo test -p hmcl` 守着）：TaskListPage、BlueprintListPage、
    DataPackListPage、ResourcePackListPage、ShaderListPage、InstallerListPage、ScreenshotsPage、
    GameInstancePage、InstanceFoldersPage；
  - 转换脚本的三条额外规则：删掉 `Click="Handler"` 这类指向 code-behind 的事件属性（运行时加载器
    找不到方法，Rust 侧改为挂闭包）；把 `{StaticResource XxxConverter}` 改写成
    `{x:Static util:XxxConverter.Instance}`（配合 `ferro_markup_type!` 的 `fields:` 声明静态字段，
    并注册 `Rc<Converter> → Rc<dyn IValueConverter>` 的值转换）；文档用到的命名空间自动补声明；
  - 仍需 Rust 化：自定义控件（15 个）、code-behind 事件处理器（运行时加载器不能按字符串找处理器，
    需在 Rust 里挂闭包）、以 `Window` 为根的对话框与需要渲染器的页面（测试环境无窗口/渲染器）、
    `DownloadViewModel`/`SettingsViewModel`/`LoginViewModel`/`ModsViewModel` 等大视图模型、
    核心模型（`Account`/`GameDirectory`/`VersionMetadata`/`RemoteAddon`/`JavaRuntime` 等）。
  - 控件集已核对：C# 用到的 ListBox/MenuItem/ContextMenu/ComboBox/ScrollViewer/DockPanel/WrapPanel/
    NumericUpDown/UserControl/TabControl/Path/Image/ProgressBar/TextBox/TrayIcon/NativeMenu 在 FerroUI 均有；
  - i18n：Java 的 `.properties` 直接装入资源字典供 `{DynamicResource}` 引用；C# 页面用的是旧版 Java
    的 `hmcl.*` 键，`tools/build_legacy_resources.py` 把 C# 的中文/英文资源整理为
    `resources/lang/legacy*.properties` 作为附加层（顺带修好了 C# 分支英文资源整体错位一格的问题）；
  - 已跑通：Rust 窗口加载 C# 的 `TaskListPage`，标题/按钮/提示文本、绑定（`RunningCount` + `StringFormat`）、
    主题配色（`HMCL.PageBackground` 等）全部正确显示，截图验证通过。
- 验收：页面结构与交互对标 Java 版；关键流程可用。

- **HTTP 层（DL-01/DL-02/AUTH-*/MP-01/K-04 的网络部分）**:
  - 选型：应用层 `hmcl` 引入 `ureq` 2.12.1（`default-features = false` + `tls`/`gzip`/`socks-proxy`）。
    它是同步客户端，与 `Fetcher` trait 的 `&mut dyn Write` 回调签名一致（`reqwest` 是异步的、会强行引入
    tokio；`hyper`/`rustls` 太底层，重定向/代理/TLS 都得自己拼）。`tls` 自带 rustls + webpki 根证书，
    不依赖系统证书库；`socks-proxy` 让玩家的 SOCKS 代理可用。
  - 纯逻辑留在 `hmcl-core::util::http`（core 保持无网络依赖）：
    `USER_AGENT`（`HMCL/<version>`）、`CONNECT_TIMEOUT`/`READ_TIMEOUT`（各 10 秒，对应 Java
    `NetworkUtils.TIMEOUT`）、`MAX_REDIRECTS = 20`；
    `ProxySetting`（`System`/`Direct`/`Custom{scheme,host,port,user,password}`）与 `to_url()`，
    对应 Java `ProxyManager`；`proxy_setting(&ProxyAddress, Option<&ProxyCredentials>)`
    从 `LauncherSettings` 取代理设置（空用户名+空密码视为无凭证，与 Java 一致）；
    `default_headers(language_tag)` 生成 `User-Agent` + `Accept-Language`（对应 Java
    `NetworkUtils.createConnection`）；`resolve_redirect(base, location)` 手工跟随重定向，
    `is_redirect_status` 为 300..=308 去掉 304/306，超过 20 次拒绝、拒绝非 HTTP 协议，
    未编码的 `Location` 按 curl 风格转义（路径空格→`%20`、查询里的 `+` 保留）。
  - `hmcl::http_fetcher::HttpFetcher` 实现 `hmcl_core::download::Fetcher`：
    `build_agent` 关掉客户端自己的重定向（`redirects(0)`）改由 `fetch` 手工跟随，设连接/读取超时、
    `User-Agent`，并按 `proxy_setting` 配置 `try_proxy_from_env` 或 `ureq::Proxy`
    （代理写错只告警、退化为直连，对应 Java 的 `NO_PROXY` 回退）；
    `fetch` 先发 `default_headers` 再叠加调用方头（ETag/`range` 等），
    `ureq::Error::Status(code, response)` 拆出非 2xx 的真实响应而不是当成连接失败，
    重定向读 `location` 交给 `resolve_redirect`，正文用 64 KiB 缓冲写进 `sink`
    （`Interrupted` 重试）；`headers_of` 在 `into_reader()` 消费响应前用
    `headers_names()` + `header(name)` 取「名字 → 值」（`headers_names()` 只给小写名字，必须再取值），
    `ResponseInfo::header()` 因此不区分大小写；`language_tag(code)` 把 `_` 换成 `-`（`zh_Hans`→`zh-Hans`）。
  - `Launcher::fetcher()` 经 `HttpFetcher::shared(settings, language)` 返回全程序共享实例，
    对应 Java `ProxyManager.init()` 在启动时按设置装好代理；代理设置的改动要重启才生效，与 Java 一致。
  - 测试：`hmcl` crate 12 个新单测（正文落盘、手工重定向、未编码 Location 转义、
    拒绝 `file://` 重定向、拒绝空 Location、请求头透传、304/404 应答、**应答头取值**等），
    全部打本地 `TcpListener` 假服务器，不访问外网。
  - 未接入：下载页/版本清单/远程整合包这些真正发请求的页面（HTTP 客户端本身已可用）。

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
- `CONT-08`（更新）依赖 `CONT-09`（元数据解析）与远程仓库；核心（检查 + 批量替换）已就绪，
  剩余部分是更新页 UI。
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
