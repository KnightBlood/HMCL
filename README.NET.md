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

界面使用 Avalonia 的 AXAML 描述，`MainWindow` 承载 `MainView`；`MainView` 由左侧导航
（游戏列表、下载、多账户、设置、更多）和右侧内容区组成。

## 构建

需要 .NET 10 SDK。

```shell
dotnet build HMCL.NET.slnx
dotnet run --project src/HMCL.NET
```
