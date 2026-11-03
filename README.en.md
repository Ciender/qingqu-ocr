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

Built with Scintilla, system Rich Edit, and GDI, the application runs directly after extraction. **OCR uses your configured cloud provider and requires network access and API permissions.** Original project materials are licensed for noncommercial purposes.

## Features

| Feature | Capabilities |
|---|---|
| Screen OCR | Freeze the current monitor, preselect a window or drag a region; recognize text, tables, vertical text, and formulas |
| Translation | Side-by-side source and translation, manual or automatic requests, target-language switching, and streaming for compatible APIs |
| Text editing | Line numbers, wrapping, search, selections, font controls, and separate source/translation undo and redo |
| Formatting | Automatic paragraphs, original OCR lines, merged paragraphs, smart cleanup, punctuation conversion, Simplified/Traditional Chinese, indentation, and paragraph spacing |
| Formulas and Markdown | Edit source on the left and preview natively on the right; copying preserves LaTeX / Markdown source |
| Tables | Multiple tables, empty cells, merged spans, captions, and footnotes; copy into Word / Excel for further editing |
| Notes and images | `Alt+V` opens the current clipboard content; images support zoom, rotation, dragging, pinning, information bars, and independent OCR |
| Task routing | Independent service and endpoint bindings for text, tables, vertical text, formulas, translation, and images |
| Desktop integration | Tray, global shortcuts, floating toolbar, startup option, single instance, and light/dark VS Code Modern / Catppuccin palettes |

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
    <td align="center"><a href="docs/screenshots/clipboard-windows.png"><img src="docs/screenshots/clipboard-windows.png" width="320" alt="Pinned image with a fine border, soft shadow, and native context menu"></a><br><strong>Pinned image and context menu</strong></td>
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

## Build from source

Requires **Rust MSVC, Visual Studio C++ Build Tools with C++17, and Windows SDK**. The current build uses Rust 1.97.1; no minimum supported Rust version is declared.

In PowerShell with the MSVC development environment configured:

```powershell
cargo build --release --locked
```

Output: `target/release/qingqu-ocr-native.exe`. Required Scintilla sources are included in `vendor/scintilla` and linked statically. The first Cargo dependency fetch requires network access.

<details>
<summary><strong>Checks, packaging, and source layout</strong></summary>

```powershell
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked -- -D warnings
pwsh -File scripts/package.ps1
```

Package the release with PowerShell. A Windows x64 ZIP and SHA-256 file are generated in `dist/`, containing the current executable, bilingual READMEs, screenshots, and applicable licenses.

| Path | Contents |
|---|---|
| `src/native/` | Win32 windows, Scintilla editor, settings, tables, and image UI |
| `src/config/`, `src/ocr/`, `src/ocr_cloud/` | Task bindings, recognition settings, and service adapters |
| `src/platform/`, `src/text/` | Capture, clipboard, text conversion, and formatting |
| `assets/`, `vendor/` | Icons, themes, native components, and local rendering adapter |
| `licenses/`, `docs/screenshots/` | Third-party notices and UI examples |

</details>

## Feedback and contributions

Include Windows / application versions, DPI, provider/model, reproduction steps, and expected behavior when reporting an issue. Attach a minimal nonsensitive sample. Do not submit API keys, `credentials.bin`, or private source documents. Read project and third-party licenses before contributing and run the basic checks above.

## License

Original code, documentation, and original artwork that the author has authority to license use **[PolyForm Noncommercial 1.0.0](LICENSE)**. **Commercial use of those materials is not granted.** Source availability is subject to that license's permitted purposes.

Third-party components retain their own licenses and the commercial-use rights those licenses grant. Project noncommercial terms do not replace third-party MIT, Apache-2.0, BSD, or other applicable licenses. Umi-OCR / GapTree-derived portions retain MIT attribution. Preserve applicable notices in [NOTICE.txt](NOTICE.txt), [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), and [licenses/](licenses/) when distributing.

Thanks to the maintainers of Scintilla, Umi-OCR / GapTree, fast_image_resize, Fluent UI System Icons, VS Code, Catppuccin, and the Rust ecosystem.
