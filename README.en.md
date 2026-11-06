<p align="center">
  <img src="assets/app.svg" width="72" height="72" alt="Qingqu OCR icon">
</p>

<h1 align="center">Qingqu OCR</h1>

<p align="center">Capture text · Translate side by side · Format paragraphs · Pin images</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-10%20%2F%2011%20x64-0078D4" alt="Windows 10 / 11 x64">
  <img src="https://img.shields.io/badge/version-0.3.0-2563EB" alt="Version 0.3.0">
  <img src="https://img.shields.io/badge/Rust-Win32-DEA584" alt="Rust + Win32">
  <img src="https://img.shields.io/badge/license-Noncommercial-64748B" alt="Noncommercial license">
</p>

<p align="center">
  <a href="README.md">简体中文</a> ·
  <a href="#getting-started">Getting started</a> ·
  <a href="#screenshots">Screenshots</a> ·
  <a href="#build-from-source">Build</a> ·
  <a href="#license">License</a>
</p>

Qingqu OCR is a Windows text recognition tool built with **Rust + Win32**. Capture screen content, edit and translate text, format paragraphs, preview formulas, and copy tables in one workspace. Clipboard text and images can also become independent desktop notes and pinned windows.

**OCR uses your configured cloud provider and requires network access and API permissions.** Original project materials are licensed for noncommercial purposes.

## Features

- **Screen OCR:** text, tables, vertical text, and formulas.
- **Translation:** two panes, multiple providers, automatic and streaming output.
- **Text tools:** editing, search, paragraphs, formatting, and Chinese/punctuation conversion.
- **Documents:** formula / Markdown previews and editable table copying to Word / Excel.
- **Clipboard windows:** text notes, pinned images, and independent OCR.
- **Desktop integration:** tray, shortcuts, floating toolbar, and light/dark themes.

## Screenshots

<table>
  <tr>
    <td align="center"><a href="docs/screenshots/workspace.png"><img src="docs/screenshots/workspace.png" width="320" alt="Two-pane source and result workspace"></a><br><strong>Two-pane workspace</strong></td>
    <td align="center"><a href="docs/screenshots/formula-preview.png"><img src="docs/screenshots/formula-preview.png" width="320" alt="Formula source and native preview"></a><br><strong>Formula preview</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/tables.png"><img src="docs/screenshots/tables.png" width="320" alt="Recognized text and table structure"></a><br><strong>Table recognition</strong></td>
    <td align="center"><a href="docs/screenshots/task-bindings.png"><img src="docs/screenshots/task-bindings.png" width="320" alt="Task, API service, and endpoint bindings"></a><br><strong>Task bindings</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/dark-theme.png"><img src="docs/screenshots/dark-theme.png" width="320" alt="Formula preview in a dark theme"></a><br><strong>Dark theme</strong></td>
    <td align="center"><a href="docs/screenshots/clipboard-windows.png"><img src="docs/screenshots/clipboard-windows.png" width="320" alt="Clipboard text note, pinned image, and image context menu"></a><br><strong>Text notes and pinned images</strong></td>
  </tr>
</table>

## Getting started

1. Extract `qingqu-ocr-0.3.0-windows-x64.zip` and run `qingqu-ocr.exe`. Fully exit the previous version from the tray before updating.
2. Open **Settings → API services** (`设置 → API 服务`). Enter your provider credentials and the required region, model, or endpoint.
3. Open **Settings → Task bindings** (`设置 → 调用配置`). Choose the task, service, and endpoint, then save and apply.
4. Press **`Alt+Q`** to capture text, or **`Alt+V`** to open clipboard text / images.

Image OCR can follow the text configuration or use its own binding. Cloud services may charge for requests; the application does not supply API keys. Available endpoints, languages, and account permissions depend on the provider.

### Supported services

<table>
  <thead><tr><th>Category</th><th>Service</th><th>Capabilities</th></tr></thead>
  <tbody>
    <tr><th colspan="3" align="left">OCR recognition</th></tr>
    <tr><td rowspan="6">OCR</td><td>Baidu OCR</td><td>Text, tables, vertical text, formulas, and document analysis</td></tr>
    <tr><td>Tencent Cloud OCR</td><td>Text, tables, and vertical text</td></tr>
    <tr><td>Alibaba Cloud standalone OCR</td><td>General text, high-accuracy documents, tables, and formulas</td></tr>
    <tr><td>Alibaba Cloud unified OCR</td><td>Text, handwriting, multilingual text, and tables</td></tr>
    <tr><td>Model Studio Qwen-OCR / Qwen-VL</td><td>Text, tables, vertical text, and formulas according to model capabilities</td></tr>
    <tr><td>Mathpix</td><td>Formula recognition</td></tr>
    <tr><th colspan="3" align="left">Text translation</th></tr>
    <tr><td rowspan="7">Translation</td><td>Google</td><td>Web service / official API</td></tr>
    <tr><td>Microsoft</td><td>Web service / official API</td></tr>
    <tr><td>Index Translate</td><td>Separate endpoint, model, and key settings</td></tr>
    <tr><td>OpenAI-compatible</td><td>Custom compatible endpoints, models, and streaming</td></tr>
    <tr><td>Baidu Translate</td><td>Official translation API</td></tr>
    <tr><td>Alibaba Cloud Translate</td><td>Official translation API</td></tr>
    <tr><td>Tencent Translate</td><td>Official translation API</td></tr>
  </tbody>
</table>

Use the **“?” button** in the application for each service's endpoint, configuration, and quota information.

### Default shortcuts

| Shortcut | Action |
|---|---|
| `Alt+Q` | Capture text from the screen |
| `Alt+V` | Open current clipboard text / images |

Assign shortcuts to other actions in Settings as needed.

<details>
<summary><strong>Formatting, tables, and image workflows</strong></summary>

- **Paragraphs:** Automatic mode uses reading order, alignment, spacing, and reliable provider hints. Split mode retains OCR lines; merge mode removes all line breaks. Smart formatting separately processes current text or selection without changing the default mode for later OCR. Externally pasted text can be merged into one line and restored or reorganized with paragraph controls.
- **Text protection:** Formatting protects code, formulas, URLs, and related structures. Settings control indentation and paragraph spacing. Manual edits are not overwritten by stale recognition content.
- **Table copying:** Whole tables and selections provide RTF, Excel XML Spreadsheet, and TSV, preserving cell line breaks, merged spans, leading zeros, and long identifiers. Review provider mistakes, missing cells, and incorrect merges.
- **Formulas / Markdown:** Common fractions, roots, scripts, headings, lists, and code are supported. Math delimiters include `$...$`, `$$...$$`, `\(...\)`, and `\[...\]`. HTML is displayed literally; scripts and remote images are not loaded. Complex TeX macros are not guaranteed. Previews exceeding 256 KiB or 8192 formatting spans fall back to source.
- **Pinned images:** Manual scale ranges from 1–800%, with smooth wheel zoom, rotation, fit-to-screen, pinning, information bars, and explicit OCR. Display zoom does not change original pixels. Recognition uses the full original with its current rotation; copying excludes borders and information bars.
- **Image copying:** Lossless PNG, DIBV5, file-list, and path-text formats are offered together. The receiving application chooses a supported format; a text-only target may receive the PNG path.
- **Sessions:** Content and undo histories remain while the app is in the tray and end when the process exits. Identical images can share the original while retaining per-window rotation.

</details>

## Data and privacy

OCR images and translation text are sent to your chosen services. Automatic translation also sends recognized text to the configured translation provider. **Opening clipboard notes or images with `Alt+V` does not itself upload content.**

Preferences are stored at `%LOCALAPPDATA%\QingquOCR\settings.json`; Windows DPAPI encrypts credentials in `credentials.bin` in the same directory. `QINGQU_DATA_DIR` selects a separate data directory. Keys can be shown or hidden in Settings; do not share credential files or screenshots containing keys.

<details>
<summary><strong>Local images and temporary files</strong></summary>

Original images and undo records use session temporary files. An original is removed after its final window/task reference is released. Copying an image leaves a PNG in the system temporary directory `QingquOCR-clipboard` so its path remains usable after exit. Later successful copies remove files older than seven days and aim for a cache limit of 128 images / 128 MiB. Current or locked files may temporarily exceed those limits. Copied sensitive images can therefore remain after exit.

</details>

## Lightweight design

The current Release build, measured on this machine with an idle two-pane workspace and floating toolbar on an isolated desktop and no cloud requests, used about **33.3 MiB working set**, **5.0 MiB private working set**, and **6.5 MiB private commit**. Two three-second CPU samples both read **0.0%**, normalized to one logical core. Working set includes shared system pages; images and recognition create additional temporary peaks. The application does not trim its working set.

- **Use system UI infrastructure.** Statically linked Scintilla handles text editing, system Rich Edit handles formulas and documents, and Win32 / GDI draws the UI. Previews are created on demand, without a resident browser or GPU rendering context.
- **Share originals and bound display caches.** Identical images share immutable file-backed storage while each window retains its own scale and rotation. Originals are mapped on demand, stationary display caches share a global **4 MiB** budget, and resources are released after the last window/task reference ends.
- **Limit rendering to visible work.** SIMD convolution and bounded tiles reuse canvas and filter buffers. Continuous zoom coalesces input around a stable sampling anchor; animation timers and temporary buffers are released when interaction stops.
- **Control concurrency and idle work.** Local processing, OCR, and translation use three fixed worker channels. Idle network clients release connections while valid authentication tokens remain reusable. Windows messages drive the UI instead of idle polling.
- **Optimize size and hot paths separately.** Release builds use size optimization, Thin LTO, one codegen unit, and symbol stripping; image convolution and its adapter receive separate speed optimization.

## Build from source

### Requirements

| Item | Requirement |
|---|---|
| Rust / Cargo | **1.97.1 or newer stable**; both the application and local renderer declare `rust-version = "1.97.1"` |
| Target | `x86_64-pc-windows-msvc`, Rust 2021 Edition |
| C++ toolchain | Visual Studio 2022 Build Tools or newer, with Desktop development with C++, MSVC x64/x86 tools, and C++17 support |
| Windows SDK | Windows 10 / 11 SDK; the current build uses **10.0.26100.0** |
| Release packaging | PowerShell **7.0+**; compiling the executable needs no additional packaging tool |

Rust 1.97.1 is the currently declared and verified minimum supported version. See the [official rustup MSVC instructions](https://rust-lang.github.io/rustup/installation/windows-msvc.html) for installing the native toolchain.

### Install the toolchain and fetch dependencies

Install [rustup](https://rustup.rs/) and the C++ tools above, then run these commands from the repository root:

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal --component rustfmt --component clippy
rustc +stable-x86_64-pc-windows-msvc --version
cargo +stable-x86_64-pc-windows-msvc fetch --locked --target x86_64-pc-windows-msvc
```

Confirm that Rust is at least 1.97.1. The first fetch needs network access. [Cargo.lock](Cargo.lock) fixes the dependency versions, while [Cargo.toml](Cargo.toml) declares runtime and build dependencies. Cargo installs the Rust libraries automatically.

### Compile and run

Use **Developer PowerShell for Visual Studio**, or PowerShell with the MSVC environment configured:

```powershell
cargo +stable-x86_64-pc-windows-msvc build --release --locked
.\target\release\qingqu-ocr-native.exe
```

The build produces `target/release/qingqu-ocr-native.exe`, distributed as `qingqu-ocr.exe`. `.cargo/config.toml` enables static CRT, and a C++17 compiler builds Scintilla into the executable. Once dependencies are cached, add `--offline` to build without network access.

### Checks and packaging

```powershell
cargo +stable-x86_64-pc-windows-msvc fmt --all -- --check
cargo +stable-x86_64-pc-windows-msvc check --locked
cargo +stable-x86_64-pc-windows-msvc clippy --locked -- -D warnings
pwsh -File scripts/package.ps1
```

The packager reads the compiled executable and locked dependencies, verifies third-party notices, and creates a Windows x64 ZIP and SHA-256 file in `dist/`. The package includes the executable, bilingual READMEs, screenshots, and licenses. Existing ZIPs are not overwritten.

<details>
<summary><strong>Dependencies and source layout</strong></summary>

Cargo retrieves registry libraries. The following local materials are already included:

- `vendor/scintilla/`: Scintilla 5.6.9 core, headers, and Win32 sources.
- `vendor/picture-renderer/`: the project's Rust image-rendering adapter.
- `vendor/text-layout/`: attribution and licenses for adapted algorithms; their Rust implementation is in `src/text/`.
- `assets/`: SVGs, themes, application icons, and Windows manifest used by the build.

Key dependencies and purposes, using versions from the current lockfile:

| Purpose | Dependencies |
|---|---|
| Windows interfaces | `windows 0.62.2`, `windows-sys 0.61.2` |
| Images and clipboard | `image 0.25.10`, `fast_image_resize 6.1.0`, `arboard 3.7.0` |
| Network and async requests | `reqwest 0.13.5`, `tokio 1.54.1`, `async-openai 0.42.3` |
| Document parsing | `pulldown-cmark 0.13.6`, `quick-xml 0.41.0` |
| Formatting and conversion | `ferrous-opencc 0.4.0`, `regex 1.13.1`, `textwrap 0.16.4`, `unicode-segmentation 1.13.3` |
| Configuration and data | `serde 1.0.229`, `serde_json 1.0.154`, `secrecy 0.10.3`, `base64 0.22.1`, `md5 0.8.1`, `futures-util 0.3.34` |
| Build resources | `cc 1.10.0`, `resvg 0.49.0`, `winresource 0.1.34` |

Use `cargo tree --locked` to inspect all transitive dependencies. Registry crate sources are not all copied into this repository; Cargo lock metadata and `licenses/` retain version, origin, and license information.

| Path | Contents |
|---|---|
| `src/native/` | Main window, notes, images, settings, and editor |
| `src/config/`, `src/ocr/`, `src/ocr_cloud/` | Task configuration and recognition adapters |
| `src/platform/`, `src/text/` | Capture, clipboard, formatting, and conversion |
| `licenses/`, `docs/screenshots/` | Third-party notices and UI examples |

</details>

## Feedback and contributions

Include Windows / application versions, DPI, provider/model, reproduction steps, and expected behavior when reporting an issue. Attach a minimal nonsensitive sample. Do not submit API keys, `credentials.bin`, or private source documents. Read project and third-party licenses before contributing and run the basic checks above.

## License

Original code, documentation, and original artwork that the author has authority to license use **[PolyForm Noncommercial 1.0.0](LICENSE)**. **Commercial use of those materials is not granted.** Source availability is subject to that license's permitted purposes.

Third-party components retain their own licenses and the commercial-use rights those licenses grant. Project noncommercial terms do not replace third-party MIT, Apache-2.0, BSD, or other applicable licenses. Umi-OCR / GapTree-derived portions retain MIT attribution. Preserve applicable notices in [NOTICE.txt](NOTICE.txt), [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), and [licenses/](licenses/) when distributing.

Thanks to the maintainers of Scintilla, Umi-OCR / GapTree, fast_image_resize, Fluent UI System Icons, VS Code, Catppuccin, and the Rust ecosystem.
