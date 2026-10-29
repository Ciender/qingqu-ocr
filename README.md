<div align="center">

<img src="assets/app.svg" width="88" alt="轻取 OCR 图标">

# 轻取 OCR · Qingqu OCR

**截图取字、翻译、排版与图片悬浮，一套原生 Windows 工作流。**

[English](README.en.md) · [功能概览](#功能概览) · [界面示范](#界面示范) · [快速开始](#快速开始) · [从源码构建](#从源码构建) · [许可证](#许可证)

![平台](https://img.shields.io/badge/Windows-10%20%2F%2011%20x64-0078D4)
![版本](https://img.shields.io/badge/version-0.3.0-2563EB)
![技术](https://img.shields.io/badge/Rust-Win32%20Native-DEA584)
![许可](https://img.shields.io/badge/license-Noncommercial-64748B)

</div>

轻取 OCR 是一款面向 Windows 10/11 x64 的桌面 OCR 工具。将屏幕内容转成可编辑文字，继续完成翻译、段落整理、公式预览、表格复制，或把剪贴板中的文字和图片固定在桌面上。

界面使用 Rust、Win32、Scintilla、系统 Rich Edit/TOM2 和 GDI 实现。Scintilla 静态编译进程序，无需另装编辑器 DLL；不内置浏览器、WebView、Python、OCR 模型或 TeX 运行时。**OCR 使用你配置的云服务，识别需要网络和相应服务权限。**

> 本项目原创部分仅限非商业用途，采用 [PolyForm Noncommercial 1.0.0](LICENSE)。第三方组件保留原许可证，详见[许可范围](#许可证)。

## 功能概览

| 模块 | 已实现的能力 |
|---|---|
| 截图识别 | 鼠标所在显示器的冻结截图、窗口预选、拖动框选、整屏识别；Esc 或右键取消 |
| 识别任务 | 普通文字、表格、竖排、公式；语言选项按接口能力提供 |
| 服务配置 | 分别保存 API 服务连接；独立绑定文字、表格、竖排、公式、翻译、图片任务 |
| 翻译 | 手动翻译、自动翻译、目标语言切换、兼容接口的流式输出；原文与译文双栏显示 |
| 文本编辑 | Scintilla 原生编辑、逻辑行号、自动折行、查找、选区操作、撤销与重做、字号调整 |
| 段落处理 | 自动分段、保留原始行、合并段落；根据坐标、对齐、行距和阅读顺序整理 OCR 结果 |
| 智能排版 | 当前文本或选区的空白清理、段内折行修复；保护代码、公式、网址等结构 |
| 文字转换 | 中英文标点切换、简繁转换、首行缩进、段间空行等输出选项 |
| 公式与文档 | 保留 LaTeX 源码；在右栏预览公式或 Markdown；复制预览时保留源码 |
| 表格 | 保留多表、空白格、合并跨度、明确返回的标题脚注；复制 RTF、Excel XML Spreadsheet 和 TSV |
| 剪贴板便签 | `Alt+V` 将当前文字打开为独立便签，或将当前图片打开为独立浮窗 |
| 图片浮窗 | 指针中心平滑缩放、旋转、适应屏幕、置顶、自由拖动、独立 OCR、可选图片信息栏 |
| 图片复制 | 同时提供 PNG、DIBV5、文件列表和路径文字；复制完整原图，不包含信息栏 |
| 桌面集成 | 系统托盘、可配置全局热键、悬浮工具条、可选开机启动、单实例保护 |
| 外观 | VS Code Modern、Catppuccin 两组浅深配色，SVG 图标与 DPI 缩放 |
| 会话 | 原文与译文独立撤销；托盘保留当前内容；图片共享原图并分别记录旋转状态 |

## 界面示范

以下均为实际程序界面的示例截图。画布统一为 **1440 × 960**，窗口保持原始比例，以留白适配尺寸；示例内容用于说明界面，不作为云端识别准确率或速度评测。当前应用界面以中文为主。

### 原文与译文

![原文与译文双栏工作区](docs/screenshots/workspace.png)

### 公式与文档预览

![公式源码与原生预览](docs/screenshots/formula-preview.png)

### 表格识别结果

![表格文字与原生表格预览](docs/screenshots/tables.png)

### 功能与 API 绑定

![按功能选择服务和具体接口](docs/screenshots/task-bindings.png)

### 深色界面

![深色主题下的工作区](docs/screenshots/dark-theme.png)

### 便签与图片浮窗

![独立便签和图片浮窗](docs/screenshots/clipboard-windows.png)

## 快速开始

### 运行环境

- Windows 10 或 Windows 11，x64。
- 使用 OCR 或在线翻译时，需要能够连接所配置服务的网络。
- 云 API 需要你自己的凭据及已开通的服务权限。程序不提供 API Key，也不承诺免费额度。
- Word、Excel 仅用于接收可编辑表格；程序本身不要求安装 Office。

### 安装与首次配置

1. 从本仓库的 **Releases** 获取 Windows x64 ZIP 发布包，解压后运行 `qingqu-ocr-native.exe`，不需要安装器。
2. 更新前，从托盘彻底退出旧版，再启动新版本。请保留发布包中的许可证和第三方声明。
3. 打开 **设置 → API 服务**，填写所需服务的密钥、地域、模型或自定义地址。
4. 打开 **设置 → 调用配置**，按“功能 → API 服务 → 具体接口”绑定任务，然后保存并应用。
5. 按 `Ctrl+Shift+O` 框选屏幕内容。识别结果进入左栏，按需翻译、排版、预览或复制。

图片识别既可以跟随普通文字配置，也可以独立绑定接口。切换文字识别服务不会替换表格、公式等其他任务的绑定。

### 常用工作流

| 场景 | 操作 |
|---|---|
| 截图取字 | `Ctrl+Shift+O` → 单击预选窗口，或拖动框选 → 松开识别 |
| 截图翻译 | 配置翻译服务 → 勾选自动翻译 → 截图识别；也可点击翻译按钮单次执行 |
| 整理粘贴文字 | 粘贴到左栏 → 点击智能排版；外部粘贴内容会合并为一行，可用分段按钮恢复或整理 |
| 提取公式 | 使用公式识别任务 → 左栏保留 LaTeX 源码，右栏显示预览 |
| 提取表格 | 使用表格识别任务 → 核对表格 → 复制整表或选区 → 粘贴到 Word / Excel |
| 固定参考内容 | 复制文字或图片 → `Alt+V` → 打开独立便签或图片浮窗 |
| 识别浮窗原图 | 图片右键 → 识别图片；识别完整原图并保留旋转方向 |

## 服务支持

下表描述本版本的客户端接入范围。具体模型、地域、语言、额度和账户权限以服务商及设置中的能力选项为准。

| 服务 | 客户端支持范围 | 配置提示 |
|---|---|---|
| 百度 OCR | 文字、表格、竖排、公式及文档分析相关接口 | 使用 API Key / Secret Key；旧公式接口的可用性需核对服务商公告 |
| 腾讯云 OCR | 文字、表格、竖排相关接口 | 使用 SecretId / SecretKey 及地域 |
| 阿里云 OCR 独立接口 | 通用文字、高精度文档、表格、公式等已适配接口 | 使用传统云 AccessKey；与百炼 API Key 分开 |
| 阿里云 OCR 统一识别 | 按任务选择文字、手写、多语言或表格等统一类型 | 任务绑定分别保存，不覆盖其他任务 |
| 百炼 Qwen-OCR / Qwen-VL | 已适配的文字、表格、竖排、公式任务；原生或兼容协议 | 配置 API Key、模型、地域、工作空间及协议；能力随模型而异 |
| Mathpix | 公式识别 | 配置对应应用凭据 |

翻译提供 **Google、Microsoft、Index Translate、OpenAI 兼容、百度、阿里云、腾讯** 七个入口。Google / Microsoft 可选择默认网页服务或已适配的官方 API；网页服务可能因服务方调整而失效。Index 与自定义兼容服务分别保存地址、模型和密钥。程序不会自动切换到另一个付费服务。

设置中的问号卡片提供当前接口的资料和官方链接。这些内容不查询账户余额，也不保证服务商的额度或价格始终不变。`Unlimited-OCR-2` 仅保留历史配置字段，当前未接入识别调用。

## 快捷键与窗口操作

| 快捷键 / 操作 | 默认行为 |
|---|---|
| `Ctrl+Shift+O` | 窗口预选或区域截图识别 |
| `Ctrl+Shift+Q` | 显示主窗口 |
| `Alt+V` | 将当前剪贴板文字 / 图片打开为独立窗口 |
| `Esc` | 主窗口最小化；截图取消；图片或子窗口关闭 |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | 文本复制 / 剪切 / 粘贴 |
| `Ctrl+Z` / `Ctrl+Y` | 撤销 / 重做 |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | 调整编辑字号 / 恢复 14 号 |
| 图片滚轮 | 以鼠标下方原图位置为锚点缩放 |
| 图片右键 | 复制、旋转、输入比例、适应屏幕、置顶、信息栏、OCR 或关闭 |
| 托盘 → 退出 | 彻底关闭所有窗口和后台进程 |

全局热键可在设置中修改。主窗关闭默认收进托盘。会话撤销记录只在当前进程中保留，彻底退出后结束。

## 识别结果与显示规则

**段落。** 自动分段结合多栏阅读顺序、局部行距、对齐和可靠的接口提示。缺少坐标时保留已有逻辑行；拆分模式保留原始行，合并模式明确去掉全部换行。智能排版是独立操作，不改变后续 OCR 的默认分段模式。

**表格。** 表格结构来自服务端返回的 JSON。复制保留合并跨度、单元格内换行、前导零和长编号；结果仍需核对服务端的错字、漏格及错误合并。左栏 TSV 支持横向滚动，右栏为原生表格预览；复制到 Word / Excel 后可继续编辑。

**公式与 Markdown。** 公式使用 Windows Rich Edit/TOM2，Markdown 使用 pulldown-cmark 解析。支持常见根式、分数、上下标及标题、列表、代码等；支持 `$...$`、`$$...$$`、`\(...\)`、`\[...\]` 数学分隔符。HTML 按文字显示，不执行脚本，不加载网络图片。它不是完整 TeX 或浏览器渲染器；复杂宏的兼容性取决于系统组件。预览超过 256 KiB 或 8192 个格式片段时回退源码。

**图片。** 支持 1–800% 手动缩放，连续滚轮平滑收敛；使用 `fast_image_resize` 的运行时 SIMD 自适应 Bilinear 卷积与分块缓冲。缩放显示和完整原图分开，复制不包含边框、阴影或信息栏。窗口可部分移出桌面，同时保留可抓取区域。

## 数据、凭据与网络

- 默认配置目录为 `%LOCALAPPDATA%\QingquOCR`；`settings.json` 保存偏好，`credentials.bin` 通过 Windows DPAPI 加密。可通过 `QINGQU_DATA_DIR` 指定独立数据目录。
- 密钥配置页可以显示或隐藏密钥；不要把凭据文件或含密钥的截图提交到仓库。
- OCR 请求把所选图片发送到你绑定的服务；翻译把待翻译文字发送到相应服务。使用自动翻译时，识别后的文字会继续提交给翻译服务。
- `Alt+V` 打开便签 / 图片本身不上传内容。图片右键识别、OCR 按钮和翻译功能按用户配置发起请求。
- 图片和撤销记录使用会话临时文件；最后一个窗口或任务释放原图引用后清理。
- 图片复制为保证路径在退出后仍可用，会在系统临时目录 `QingquOCR-clipboard` 留下 PNG。后续成功复制时清理 7 天前的旧文件，并以 128 张 / 128 MiB 为缓存目标；当前或被占用文件可能暂时超过上限。敏感图片的复制文件可能在退出后继续保留。

## 从源码构建

### 前置条件

- Rust MSVC 工具链：`x86_64-pc-windows-msvc`；本版本在 Rust 1.97.1 上验证，未声明最低 Rust 版本。
- Visual Studio C++ Build Tools 和 Windows SDK；构建过程需要 C++17 编译器来静态编译 Scintilla。
- 首次获取 Cargo 依赖需要网络；依赖缓存齐全后可离线构建。
- Python 3.11+ 仅用于可选的 ZIP 打包，运行应用不需要 Python。

在已配置 MSVC 开发环境的 PowerShell 中执行：

```powershell
cargo build --release --locked
```

产物位于 `target/release/qingqu-ocr-native.exe`。仓库中的 `.cargo/config.toml` 为 Windows MSVC 构建启用静态 CRT；Scintilla 所需源码已包含在 `vendor/scintilla`。

基础检查：

```powershell
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked -- -D warnings
```

打包 EXE、README 和许可证：

```powershell
python scripts/package.py
```

打包脚本读取当前构建产物和锁定的 Windows 依赖元数据，在 `dist/` 生成 `qingqu-ocr-native-0.3.0-windows-x64.zip` 及 SHA-256 文件；不复制开发记录、测试报告或历史试验 EXE。

### 目录结构

```text
src/                     应用、服务适配和平台代码
  config/                任务绑定、识别与语言配置
  native/                Win32 主窗、设置、编辑器、表格和图片 UI
  ocr/                   输入预算、公式与电子表格处理
  platform/              截图、剪贴板和本地缓存
  text/                  排版、阅读顺序与段落规则
assets/                  应用图标、SVG、主题与 manifest
vendor/                  Scintilla、图片渲染适配及算法来源声明
licenses/                第三方许可文本和组件清单
docs/screenshots/        统一尺寸的界面示范
scripts/package.py       发布包生成
LICENSE                  原创部分的非商业许可证
NOTICE.txt               版权和许可范围
THIRD_PARTY_NOTICES.txt   第三方组件说明
```

## 已知限制

- 当前面向 Windows x64；不支持 macOS / Linux，也不提供离线 OCR 引擎。
- 界面主要为中文；英文 README 不代表应用已完成英文界面本地化。
- 截图只覆盖鼠标所在显示器，不跨屏框选。原始像素超过 48 MiB 的显示器截图会提示超限。
- 上传前按尺寸和 PNG 字节预算处理，过大的图像可能缩小，密集小字可能受影响；当前不自动分块追加识别请求。
- 云服务的模型、端点、权限和网络状态会影响识别结果；不保证固定延迟、免费额度或所有语言的准确率。
- 自定义、Qwen 和兼容协议的可用性需使用自己的账户验证；客户端支持不等于所有模型均已完成真实云端验证。
- 触控板、特定输入法、混合 DPI 多屏和复杂公式仍可能存在设备相关差异。大图缩放的帧率和内存峰值取决于 CPU、视口及图片尺寸。

## 反馈

提交问题时，请说明 Windows 版本、程序版本、DPI、所用服务 / 模型、复现步骤和预期结果。附上不包含个人信息的最小示例和必要截图；不要附带 API Key、`credentials.bin` 或原始敏感文档。

## 许可证

本项目作者有权授权的原创代码、文档及原创图形采用 **[PolyForm Noncommercial License 1.0.0](LICENSE)**，仅允许该许可证规定的用途，**不授予商业使用许可**。这是提供源码的非商业项目，不宣称属于不限制商业用途的开源许可。

第三方组件继续使用各自许可证。本项目的非商业条款不会缩减 MIT、Apache-2.0、BSD、Unicode 或 Scintilla 等原许可授予的权利。基于 Umi-OCR / GapTree 的排版部分保留其 MIT 来源和版权声明。请阅读 [NOTICE.txt](NOTICE.txt)、[THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) 以及 [licenses/](licenses/)；分发时保留适用的版权、许可和声明。

感谢 Scintilla、Umi-OCR / GapTree、fast_image_resize、Microsoft Fluent UI System Icons、VS Code、Catppuccin 及 Rust 生态的维护者。
