<p align="center">
  <img src="assets/app.svg" width="72" height="72" alt="轻取 OCR 图标">
</p>

<h1 align="center">轻取 OCR</h1>

<p align="center">截图取字 · 双栏翻译 · 智能排版 · 图片悬浮</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-10%20%2F%2011%20x64-0078D4" alt="Windows 10 / 11 x64">
  <img src="https://img.shields.io/badge/version-0.3.0-2563EB" alt="版本 0.3.0">
  <img src="https://img.shields.io/badge/Rust-Win32-DEA584" alt="Rust + Win32">
  <img src="https://img.shields.io/badge/license-Noncommercial-64748B" alt="非商业许可">
</p>

<p align="center">
  <a href="README.en.md">English</a> ·
  <a href="#开始使用">开始使用</a> ·
  <a href="#界面预览">界面预览</a> ·
  <a href="#从源码构建">构建</a> ·
  <a href="#技术栈与架构选型">技术栈</a> ·
  <a href="#许可证">许可证</a>
</p>

轻取 OCR 是一款使用 **Rust + Win32** 开发的 Windows 桌面文字识别工具：从屏幕提取文字，在同一工作区完成编辑、翻译、排版、公式预览和表格复制，也可以将剪贴板内容固定为桌面便签或图片浮窗。

**OCR 通过你配置的云服务完成，需要网络及对应 API 权限。** 原创代码采用非商业许可。

## 主要功能

- **截图识别**：文字、表格、竖排、公式。
- **双栏翻译**：多服务、自动翻译、流式输出。
- **文本整理**：编辑、查找、分段、排版、简繁与标点转换。
- **文档处理**：公式 / Markdown 预览，表格复制到 Word / Excel。
- **剪贴板窗口**：文字便签、图片浮窗、独立 OCR。
- **桌面集成**：托盘、热键、悬浮条与浅深主题。

## 界面预览

<table>
  <tr>
    <td align="center"><a href="docs/screenshots/workspace.png"><img src="docs/screenshots/workspace.png" width="320" alt="原文与结果双栏工作区"></a><br><strong>双栏工作区</strong></td>
    <td align="center"><a href="docs/screenshots/formula-preview.png"><img src="docs/screenshots/formula-preview.png" width="320" alt="公式源码与原生预览"></a><br><strong>公式预览</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/tables.png"><img src="docs/screenshots/tables.png" width="320" alt="表格原文与结构预览"></a><br><strong>表格识别</strong></td>
    <td align="center"><a href="docs/screenshots/task-bindings.png"><img src="docs/screenshots/task-bindings.png" width="320" alt="功能、API 服务与接口绑定"></a><br><strong>调用配置</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/dark-theme.png"><img src="docs/screenshots/dark-theme.png" width="320" alt="深色主题下的公式预览"></a><br><strong>深色主题</strong></td>
    <td align="center"><a href="docs/screenshots/clipboard-windows.png"><img src="docs/screenshots/clipboard-windows.png" width="320" alt="文字便签、图片浮窗及其右键菜单"></a><br><strong>文字便签与图片浮窗</strong></td>
  </tr>
</table>

## 开始使用

1. 从 [Releases 下载 Windows x64 发布包](https://github.com/Ciender/qingqu-ocr/releases/latest)，解压 `qingqu-ocr-0.3.0-windows-x64.zip`，运行 `qingqu-ocr.exe`。更新前先从托盘彻底退出旧版。
2. 打开 **设置 → API 服务**，填写你要使用的服务凭据，以及所需的地域、模型或地址。
3. 打开 **设置 → 调用配置**，按“功能 → API 服务 → 具体接口”选择各任务的绑定，保存并应用。
4. 按 **`Alt+Q`** 截图识别，或按 **`Alt+V`** 打开剪贴板文字 / 图片。

图片识别可以跟随文字配置，也可以独立设置。云服务可能收费，程序不提供 API Key；可用接口、语言及账户权限由服务商决定。

### 支持的服务

<table>
  <thead><tr><th>类别</th><th>服务</th><th>支持内容</th></tr></thead>
  <tbody>
    <tr><th colspan="3" align="left">OCR 识别</th></tr>
    <tr><td rowspan="6">OCR</td><td>百度 OCR</td><td>文字、表格、竖排、公式及文档分析</td></tr>
    <tr><td>腾讯云 OCR</td><td>文字、表格、竖排</td></tr>
    <tr><td>阿里云 OCR 独立接口</td><td>通用文字、高精度文档、表格、公式</td></tr>
    <tr><td>阿里云 OCR 统一识别</td><td>文字、手写、多语言、表格</td></tr>
    <tr><td>百炼 Qwen-OCR / Qwen-VL</td><td>按模型能力提供文字、表格、竖排与公式识别</td></tr>
    <tr><td>Mathpix</td><td>公式识别</td></tr>
    <tr><th colspan="3" align="left">文字翻译</th></tr>
    <tr><td rowspan="7">翻译</td><td>Google</td><td>网页服务 / 官方 API</td></tr>
    <tr><td>Microsoft</td><td>网页服务 / 官方 API</td></tr>
    <tr><td>Index Translate</td><td>独立地址、模型与密钥配置</td></tr>
    <tr><td>OpenAI 兼容</td><td>自定义兼容地址、模型及流式输出</td></tr>
    <tr><td>百度翻译</td><td>官方翻译 API</td></tr>
    <tr><td>阿里云翻译</td><td>官方翻译 API</td></tr>
    <tr><td>腾讯翻译</td><td>官方翻译 API</td></tr>
  </tbody>
</table>

各服务的接口、配置和额度说明，可点击软件中的 **“？”按钮** 查看。

### 默认快捷键

| 快捷键 | 操作 |
|---|---|
| `Alt+Q` | 截图识别 |
| `Alt+V` | 打开当前剪贴板文字 / 图片 |

其他功能可在设置中按需分配快捷键。

<details>
<summary><strong>排版、表格和图片的使用说明</strong></summary>

- **段落处理：** 自动分段结合阅读顺序、对齐、行距和可靠的接口信息；拆分保留原始 OCR 行，合并去掉全部换行。智能排版独立处理当前文字或选区，不改变后续识别的默认模式。外部粘贴文字可一键合并为一行，再用段落按钮恢复或整理。
- **文本保护：** 智能排版保护代码、公式、网址等结构；首行缩进、段间空行由设置控制。手工编辑后不使用旧识别内容覆盖当前文字。
- **表格复制：** 支持整表与选区，提供 RTF、Excel XML Spreadsheet 和 TSV，保留单元格换行、合并跨度、前导零及长编号。表格结构依赖云端返回结果，错字、漏格和错误合并仍需核对。
- **公式 / Markdown：** 支持常见分数、根式、上下标、标题、列表和代码。数学分隔符包括 `$...$`、`$$...$$`、`\(...\)`、`\[...\]`。HTML 按文字显示，不执行脚本或加载网络图片；复杂 TeX 宏不保证兼容。超过 256 KiB 或 8192 个格式片段时回退源码。
- **图片浮窗：** 支持 1–800% 手动比例、平滑滚轮缩放、左右旋转、适应屏幕、置顶、信息栏和主动识别。显示缩放不改变原图；识别使用完整原图并保留旋转方向，复制不包含边框或信息栏。
- **拖动与复位：** 图片各方向最多移出屏幕 95%，保留 5% 在屏内，任务栏上沿不作为拖动边界。多屏拖动允许通过屏幕交界，并排除显示器之间的空隙。托盘“Alt+V 全部剪贴板复位”将全部图片和文字便签移回鼠标所在屏幕的可用桌面，恢复最小化便签并错开摆放。
- **图片复制：** 同时提供无损 PNG、DIBV5、文件列表和路径文字，目标软件选择其支持的格式。纯文字目标可能收到 PNG 文件路径。
- **会话：** 收进托盘后保留内容与撤销记录；彻底退出后结束本次会话。相同图片可共享原图，各窗口分别记录旋转状态。

</details>

## 数据与隐私

OCR 图片和待翻译文字发送到你选择的服务；开启自动翻译后，识别结果会继续提交给翻译服务。**`Alt+V` 打开便签或图片本身不上传内容。**

偏好设置保存在 `%LOCALAPPDATA%\QingquOCR\settings.json`，凭据由 Windows DPAPI 加密，保存在同目录的 `credentials.bin`。可通过 `QINGQU_DATA_DIR` 指定独立数据目录。密钥可在配置页显示或隐藏，请勿分享含密钥的截图或凭据文件。

<details>
<summary><strong>本地图片和临时文件</strong></summary>

原图与撤销记录使用会话临时文件；原图在最后一个窗口或任务释放引用后清理。为使复制出的文件路径在退出后仍可用，图片复制会在系统临时目录 `QingquOCR-clipboard` 保留 PNG。后续成功复制时清理 7 天前的旧文件，并以 128 张 / 128 MiB 为缓存目标；当前或被占用文件可能暂时超限。复制过的敏感图片可能在退出后继续保留。

</details>

## 从源码构建

### 构建环境

| 项目 | 要求 |
|---|---|
| Rust / Cargo | **1.97.1 或更新的稳定版**；主程序和本地渲染模块均在 `Cargo.toml` 声明 `rust-version = "1.97.1"` |
| 目标平台 | `x86_64-pc-windows-msvc`，Rust 2021 Edition |
| C++ 工具链 | Visual Studio 2022 Build Tools 或更新版本，安装“使用 C++ 的桌面开发”，包含 MSVC x64/x86 工具和 C++17 支持 |
| Windows SDK | Windows 10 / 11 SDK；当前构建使用 **10.0.26100.0** |
| 发布打包 | PowerShell **7.0+**；直接构建 EXE 不需要额外打包工具 |

Rust 1.97.1 是本项目当前声明并验证的最低受支持版本。MSVC 环境安装方式见 [rustup 官方说明](https://rust-lang.github.io/rustup/installation/windows-msvc.html)。

### 安装工具链与获取依赖

安装 [rustup](https://rustup.rs/) 和上述 C++ 工具后，在仓库根目录执行：

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal --component rustfmt --component clippy
rustc +stable-x86_64-pc-windows-msvc --version
cargo +stable-x86_64-pc-windows-msvc fetch --locked --target x86_64-pc-windows-msvc
```

确认 Rust 版本不低于 1.97.1。首次获取第三方 Rust 库需要网络；具体版本由 [Cargo.lock](Cargo.lock) 锁定，运行依赖和构建依赖均在 [Cargo.toml](Cargo.toml) 声明。无需手动逐个安装 Rust 库。

### 编译与运行

在 **Developer PowerShell for Visual Studio** 或已配置 MSVC 环境的 PowerShell 中执行：

```powershell
cargo +stable-x86_64-pc-windows-msvc build --release --locked
.\target\release\qingqu-ocr-native.exe
```

产物为 `target/release/qingqu-ocr-native.exe`，发布包内统一命名为 `qingqu-ocr.exe`。`.cargo/config.toml` 启用静态 CRT；Scintilla 由 C++17 编译器静态编译进程序。依赖缓存齐全后，可给构建命令添加 `--offline` 离线编译。

### 检查与打包

```powershell
cargo +stable-x86_64-pc-windows-msvc fmt --all -- --check
cargo +stable-x86_64-pc-windows-msvc check --locked
cargo +stable-x86_64-pc-windows-msvc clippy --locked -- -D warnings
pwsh -File scripts/package.ps1
```

打包脚本读取构建产物和锁定依赖，检查第三方许可后，在 `dist/` 生成 Windows x64 ZIP 及 SHA-256 文件；包含 EXE、双语 README、示范图和许可文本。打包不会覆盖已有同名 ZIP。

<details>
<summary><strong>依赖与代码结构</strong></summary>

普通 Rust 库由 Cargo 获取；以下本地内容已随源码提供：

- `vendor/scintilla/`：Scintilla 5.6.9 的核心、头文件和 Win32 源码。
- `vendor/picture-renderer/`：本项目的 Rust 图片渲染适配模块。
- `vendor/text-layout/`：移植算法的来源和许可；Rust 实现在 `src/text/`。
- `assets/`：构建所需 SVG、主题、应用图标与 Windows manifest。

主要依赖及用途（版本对应当前锁文件）：

| 用途 | 依赖 |
|---|---|
| Windows 接口 | `windows 0.62.2`、`windows-sys 0.61.2` |
| 图片与剪贴板 | `image 0.25.10`、`fast_image_resize 6.1.0`、`arboard 3.7.0` |
| 网络与异步请求 | `reqwest 0.13.5`、`tokio 1.54.1`、`async-openai 0.42.3` |
| 文档解析 | `pulldown-cmark 0.13.6`、`quick-xml 0.41.0` |
| 排版与文字转换 | `ferrous-opencc 0.4.0`、`regex 1.13.1`、`textwrap 0.16.4`、`unicode-segmentation 1.13.3` |
| 配置与数据 | `serde 1.0.229`、`serde_json 1.0.154`、`secrecy 0.10.3`、`base64 0.22.1`、`md5 0.8.1`、`futures-util 0.3.34` |
| 构建资源 | `cc 1.10.0`、`resvg 0.49.0`、`winresource 0.1.34` |

完整传递依赖可通过 `cargo tree --locked` 查看。第三方库未全部复制进仓库；其版本、来源和适用许可分别见 Cargo 锁文件及 `licenses/`。

| 路径 | 内容 |
|---|---|
| `src/native/` | 主窗、便签、图片、设置与编辑器 |
| `src/config/`、`src/ocr/`、`src/ocr_cloud/` | 任务配置与识别服务适配 |
| `src/platform/`、`src/text/` | 截图、剪贴板、排版与转换 |
| `licenses/`、`docs/screenshots/` | 第三方声明与界面示范 |

</details>

## 技术栈与架构选型

本项目为纯原生 Windows 桌面应用，整体设计以**零 Web 运行时、零冗余中间层、编译期优化**为核心原则：

### 核心选型与轻量化考量

| 模块 | 技术选型 | 关键技术考量与实现细节 |
|---|---|---|
| **语言与运行时** | Rust 2021 + MSVC C++17 | 全静态链接 CRT（`-C target-feature=+crt-static`），无 VC++ 运行库分发依赖，产物为绿色单二进制 |
| **GUI 架构** | Win32 API (`windows-sys` / `windows`) | 纯 Win32 消息循环驱动 + GDI / DWM，无 Electron / WebView2 / Qt 等重型框架开销，空闲 CPU 恒为 0% |
| **文本编辑器** | Scintilla 5.6.9 | C++17 源码在 `build.rs` 中通过 `cc` 静态内联编译进二进制，内存开销数兆级，替代笨重的 Web 代码编辑器 |
| **公式与文档预览** | RichEdit + TOM (`ITextDocument2`) | 利用 Windows 原生 RichEdit 控件的 `TOM_CONVERT_TEX` 特性直接渲染 LaTeX 数学公式，免去 Chromium / MathJax / KaTeX 引擎 |
| **Markdown 引擎** | `pulldown-cmark` | Zero-copy 流式解析 AST 标记，直接映射到 RichEdit 的字符格式区间（Span），避免整树对象开销 |
| **图像缩放与渲染** | `fast_image_resize` + 自研瓦片裁剪 | 运行时检测并动态分发 SIMD（AVX2/SSE4.1）卷积；结合 Bounded Tiles 视口裁剪算法，仅重采样屏幕可见区域像素 |
| **版面重构算法** | GapTree (Rust 原生移植) | 基于文本块几何拓扑与阅读顺序实现段落合并与拆分，结合 `unicode-segmentation` 处理标点与边界 |
| **图标资源处理** | `resvg` (仅构建期构建依赖) | 编译期由 `build.rs` 完成 Fluent SVG 图标光栅化并烘焙为紧凑位图，运行时零矢量解析引擎负担 |

### 运行时开销与资源回收（实测数据）

| 运行状态 | 物理工作集 (Working Set) | 私有工作集 (Private WS) | 私有提交 (虚拟内存) | CPU 占用 | 句柄 / GDI / USER |
|---|---|---|---|---|---|
| **主窗静置 / 托盘后台** | **22.50 MiB** | **2.41 MiB** | **3.33 MiB** | **0.0%**（事件驱动无轮询） | GDI: 48, USER: 67, 句柄: 225 |

> 注：经 300 次窗口开关压测无句柄泄漏（GDI/USER/系统句柄均完全对齐回收），4K 图片高频缩放关窗后工作集与私有提交即刻回落至 ~19.4 MiB / 3.72 MiB 初始基线。

### 编译与产物控制

* **分层优化**：全局配置 `opt-level = "s"`（体积优先）、`lto = "thin"`、`codegen-units = 1`、`panic = "abort"`；仅对高频计算路径（`fast_image_resize`、`qingqu-picture-renderer`）单独配置 `opt-level = 3`。
* **特性剪裁**：严格禁用第三方库默认特性（`image` 仅保留 `png/jpeg/bmp` 解码器，`tokio` 仅启用 `rt/time` 异步任务微内核），单 EXE 产物控制在 ~9.6 MB（打包 ZIP 约 6.7 MB）。

## 反馈与贡献

报告问题时，请提供 Windows / 程序版本、DPI、服务与模型、复现步骤及预期结果，附不含敏感信息的最小样例。请勿提交 API Key、`credentials.bin` 或私人原始文档。提交代码前请阅读项目及第三方许可，并运行上述基础检查。

## 许可证

作者有权授权的原创代码、文档与原创图形采用 **[PolyForm Noncommercial 1.0.0](LICENSE)**，**不授予商业使用许可**。本项目提供源码，使用范围受该许可限制。

第三方组件继续适用原许可证，保留原有商用权利；项目的非商业条款不替换 MIT、Apache-2.0、BSD 等第三方许可。Umi-OCR / GapTree 派生部分保留 MIT 来源与版权声明。分发时请保留 [NOTICE.txt](NOTICE.txt)、[THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) 及 [licenses/](licenses/) 中适用的声明。

感谢 Scintilla、Umi-OCR / GapTree、fast_image_resize、Fluent UI System Icons、VS Code、Catppuccin 及 Rust 生态的维护者。
