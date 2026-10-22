# HMCL.NET

HMCL 的 C# 重写版本，使用 .NET 10、Avalonia 12.x，工程文件为 `HMCL.NET.slnx`。

## 目录结构

```
HMCL.NET.slnx                     # 解决方案（slnx 格式）
src/
├── HMCLCore.NET/                 # 与界面无关的核心库
│   ├── Metadata.cs               # 标题、版本、目录常量
│   ├── Task/Task.cs              # 任务抽象
│   └── Util/
│       ├── Logging/Logger.cs
│       └── OperatingSystem.cs
└── HMCL.NET/                     # Avalonia 桌面程序
    ├── Program.cs                # 入口
    ├── App.axaml(.cs)            # 应用与 Fluent 主题
    ├── MainWindow.axaml(.cs)     # 主窗口
    └── Views/MainView.axaml(.cs) # 侧边栏 + 内容区
```

## 界面

界面使用 Avalonia 的 AXAML 描述，`MainWindow` 承载 `MainView`；`MainView` 由顶部的标题栏、
左侧导航（账户、游戏、通用）与右侧内容区组成，左侧导航的分组与空状态与 Java HMCL 一致。

## 构建

需要 .NET 10 SDK。

```shell
dotnet build HMCL.NET.slnx
dotnet run --project src/HMCL.NET
```

### 原生 AOT 发布

启动器可以发布为不依赖 .NET 运行时的原生可执行文件。发布原生 AOT 需要
Visual Studio 的“使用 C++ 的桌面开发”工作负载（或单独安装 MSVC 工具链与 Windows SDK），
因为链接阶段需要 `link.exe`。

```shell
dotnet publish src/HMCL.NET/HMCL.NET.csproj -c Release -r win-x64
```

产物位于 `src/HMCL.NET/bin/Release/net10.0/win-x64/publish/`，其中 `HMCL.NET.exe`
是原生可执行文件，仅依赖随包提供的 SkiaSharp / HarfBuzzSharp 原生库。

为兼容 AOT，代码中不使用反射式绑定：

- XAML 全部使用编译期绑定（`AvaloniaUseCompiledBindingsByDefault`）；
- 控件之间的属性连接通过 `HMCL.NET.Util.PropertyLink` 完成，不依赖 `Binding` 的反射；
- JSON 的读写使用 `System.Text.Json` 的源生成元数据
  （`HMCLCore.NET.Util.LauncherJsonContext` 与 `SettingsJsonContext`）。

## 第三方依赖

| 包 | 许可证 | 用途 |
|---|---|---|
| `Avalonia` 12.x | MIT | 跨平台界面框架 |
| `FluentAvalonia`（Fluent 主题） | MIT | 界面主题 |
| `fNbt` | LGPL-3.0 | NBT 格式读写 |
| `SkiaSharp` | MIT | 2D 绘制（经 `MinecraftSkinRender` 间接依赖） |
| `MinecraftSkinRender.Image` 1.2.0 | MIT | 玩家皮肤渲染（3D 头部 + 全身模型），软件渲染，无需原生 GL/Vulkan 上下文 |
| `JetBrains.Annotations` | Apache-2.0 | 空值注解 |

`MinecraftSkinRender.Image` 来自 <https://github.com/Coloryr/MinecraftSkinRender>（MIT），
与本仓库的 GPLv3 许可兼容。
