# HMCL.RS

HMCL 的 Rust 重写版本，界面使用 [FerroUI](https://github.com/wieslawsoltes/FerroUI) 构建，
布局写在 XAML 中，与 Avalonia 版本的写法一致。

## 目录结构

```
hmcl-rs/
├── Cargo.toml                  # workspace，统一依赖版本
├── crates/
│   ├── hmcl-core/              # 与界面无关的核心：元数据、路径、系统检测、任务抽象
│   │   └── src/
│   │       ├── metadata.rs
│   │       ├── task/mod.rs
│   │       └── util/{logging,operating_system}.rs
│   └── hmcl/                   # 桌面程序
│       ├── src/
│       │   ├── main.rs         # 入口：配置 AppBuilder，启动桌面生命周期
│       │   ├── app.rs          # Application：安装 Fluent 主题、创建主窗口
│       │   └── ui/mod.rs       # 用 FerroUI 运行时加载器载入 XAML
│       └── ui/MainWindow.xaml  # 主窗口界面
```

## 界面

`ui/MainWindow.xaml` 使用 FerroUI 的 XAML 方言（默认命名空间
`https://github.com/ferroui`），由 `ferroui::markup_xaml_loader::FerroRuntimeXamlLoader`
在运行时加载。文档通过 `include_str!` 嵌入二进制，因此发布时不需要额外的资源文件。

后续如果需要编译期检查，可以把运行时加载换成 `ferroui-build` 在 `build.rs` 中编译 XAML。

## 构建

需要 Rust 1.89 或更高版本。Windows 需要 MSVC 构建工具；FerroUI 默认的 ANGLE 渲染后端
需要 `libclang`，首次构建会下载或从源码编译 Skia 与 HarfBuzz，耗时较长。

```shell
cargo run -p hmcl
cargo test -p hmcl-core
```
