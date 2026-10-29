<div align="center">

<img src="assets/app.svg" width="88" alt="Qingqu OCR icon">

# Qingqu OCR · 轻取 OCR

**Capture text, translate, format documents, and pin reference images on Windows.**

[简体中文](README.md) · [Features](#features) · [Screenshots](#screenshots) · [Getting started](#getting-started) · [Build from source](#build-from-source) · [License](#license)

![Platform](https://img.shields.io/badge/Windows-10%20%2F%2011%20x64-0078D4)
![Version](https://img.shields.io/badge/version-0.3.0-2563EB)
![Stack](https://img.shields.io/badge/Rust-Win32%20Native-DEA584)
![License](https://img.shields.io/badge/license-Noncommercial-64748B)

</div>

Qingqu OCR is a Windows 10/11 x64 desktop application for turning screen content into editable text. Continue with translation, paragraph cleanup, formula previews, and editable table copying, or pin clipboard text and images in independent desktop windows.

The application uses Rust, Win32, Scintilla, system Rich Edit/TOM2, and GDI. Scintilla is linked statically, with no separate editor DLL to install. The runtime does not bundle a browser, WebView, Python, OCR models, or TeX. **OCR uses the cloud service you configure and requires network access and the corresponding service permissions.**

> Original project materials are licensed for noncommercial purposes under [PolyForm Noncommercial 1.0.0](LICENSE). Third-party components retain their original licenses. See [License](#license).

## Features

| Area | Implemented capabilities |
|---|---|
| Screen capture | Frozen capture of the monitor under the pointer, window preselection, region selection, and full-monitor recognition; cancel with Esc or right-click |
| Recognition tasks | Plain text, tables, vertical text, and formulas; language controls follow endpoint capabilities |
| Service routing | Saved API connections with independent bindings for text, tables, vertical text, formulas, translation, and pinned images |
| Translation | Manual or automatic translation, target-language switching, streaming for compatible endpoints, and a two-pane source/translation workspace |
| Text editing | Native Scintilla editing, logical line numbers, wrapping, search, selections, undo/redo, and font-size controls |
| Paragraphs | Automatic paragraphs, original OCR lines, or merged text; layout-aware reading order and alignment rules |
| Formatting | Cleanup of current text or selection, conservative line joining, and protection for code, formulas, URLs, and related structures |
| Conversion | Chinese/English punctuation modes, Simplified/Traditional Chinese conversion, indentation, and paragraph-spacing preferences |
| Document previews | Editable LaTeX source with inline formula or Markdown preview; copying a preview preserves its source |
| Tables | Multiple tables, empty cells, merged spans, explicit captions/footnotes, and RTF / Excel XML Spreadsheet / TSV clipboard output |
| Clipboard notes | `Alt+V` opens current clipboard text as a note or an image as an independent pinned window |
| Pinned images | Pointer-anchored smooth zoom, rotation, fit-to-screen, always-on-top, dragging, independent OCR, and an optional information bar |
| Image copying | PNG, DIBV5, file-list, and path-text clipboard formats; full original image pixels without the information bar |
| Desktop integration | System tray, configurable global shortcuts, floating toolbar, optional startup, and single-instance protection |
| Appearance | Light and dark VS Code Modern / Catppuccin palettes, SVG icons, and DPI-aware layout |
| Session | Independent source/translation undo histories, tray persistence, and shared original images with per-window rotation |

## Screenshots

These are captures of the actual application UI with illustrative content. Every image uses a **1440 × 960** canvas; windows keep their aspect ratios and are padded to fit. The examples demonstrate the interface, not cloud recognition accuracy or performance. The application UI is currently primarily Chinese.

### Source and translation

![Two-pane source and translation workspace](docs/screenshots/workspace.png)

### Formula and document preview

![Formula source and native preview](docs/screenshots/formula-preview.png)

### Table recognition results

![Table text and native table preview](docs/screenshots/tables.png)

### Task-specific API bindings

![Independent task, service, and endpoint selection](docs/screenshots/task-bindings.png)

### Dark appearance

![Workspace in a dark theme](docs/screenshots/dark-theme.png)

### Clipboard notes and pinned images

![Independent text note and image window](docs/screenshots/clipboard-windows.png)

## Getting started

### Requirements

- Windows 10 or Windows 11, x64.
- Network access to the configured provider for OCR and online translation.
- Your own credentials and enabled cloud API permissions where required. The application does not provide API keys or guarantee free quotas.
- Word or Excel is only needed as a destination for editable tables; Office is not required to run Qingqu OCR.

### Install and configure

1. Obtain the Windows x64 ZIP from this repository's **Releases**, extract it, and run `qingqu-ocr-native.exe`. No installer is required.
2. Before updating, fully exit the previous version from the tray. Retain the license files and third-party notices in the package.
3. Open **Settings → API services** (`设置 → API 服务`) and enter the required keys, region, model, or custom endpoint.
4. Open **Settings → Task bindings** (`设置 → 调用配置`), bind each task to a service and endpoint, then save and apply.
5. Press `Ctrl+Shift+O` and select screen content. Edit recognized text on the left, then translate, format, preview, or copy it.

Pinned-image OCR can follow the text task or use an independent binding. Changing the text provider does not replace bindings for tables, formulas, or other tasks.

### Common workflows

| Goal | Steps |
|---|---|
| Capture text | `Ctrl+Shift+O` → click a preselected window or drag a region → release to recognize |
| Translate a capture | Configure translation → enable automatic translation → recognize; the translation button also runs a single manual request |
| Clean pasted text | Paste on the left → Smart formatting; externally pasted text is merged into one line, with paragraph controls available to restore or reorganize it |
| Extract formulas | Choose formula recognition → retain LaTeX on the left and inspect the preview on the right |
| Extract tables | Choose table recognition → review the structure → copy a table or selection → paste into Word / Excel |
| Pin reference material | Copy text or an image → `Alt+V` → use an independent note or image window |
| Recognize a pinned image | Right-click the image → Recognize image; the full original image is used with its current rotation |

## Provider support

This table describes the integrations implemented by this client. Available models, regions, languages, quotas, and account permissions depend on the provider and the capabilities exposed in Settings.

| Provider | Client integrations | Configuration |
|---|---|---|
| Baidu OCR | Text, tables, vertical text, formulas, and related document-analysis endpoints | API Key / Secret Key; check provider notices for legacy formula availability |
| Tencent Cloud OCR | Text, table, and vertical-text endpoints | SecretId / SecretKey and region |
| Alibaba Cloud standalone OCR | Adapted text, high-accuracy document, table, and formula endpoints | Traditional cloud AccessKey credentials, separate from Model Studio API keys |
| Alibaba Cloud unified OCR | Task-specific unified text, handwriting, multilingual, and table types | Bindings are saved independently for each task |
| Model Studio Qwen-OCR / Qwen-VL | Adapted text, table, vertical-text, and formula tasks using native or compatible protocols | API key, model, region, workspace, and protocol; capabilities vary by model |
| Mathpix | Formula recognition | Corresponding application credentials |

Translation has seven entries: **Google, Microsoft, Index Translate, OpenAI-compatible, Baidu, Alibaba Cloud, and Tencent**. Google / Microsoft support the default web-service path or adapted official APIs. Web-service paths may stop working after provider changes. Index and custom compatible services save separate URLs, models, and keys. The client does not automatically switch to another paid service.

Question-mark cards in Settings contain provider information and official links. They do not query account balances or guarantee current prices or quotas. `Unlimited-OCR-2` has legacy configuration fields only; recognition is not implemented.

## Shortcuts and windows

| Shortcut / action | Default behavior |
|---|---|
| `Ctrl+Shift+O` | Window preselection or region capture |
| `Ctrl+Shift+Q` | Show the main window |
| `Alt+V` | Open current clipboard text / image in an independent window |
| `Esc` | Minimize the main window, cancel capture, or close an image/secondary window |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste text |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Change editor font size / restore size 14 |
| Image mouse wheel | Zoom around the image point under the pointer |
| Image context menu | Copy, rotate, enter scale, fit, pin, toggle information, recognize, or close |
| Tray → Exit | Fully close all application windows and background work |

Global shortcuts are configurable. Closing the main window normally sends it to the tray. Undo histories last for the current process and end when the application fully exits.

## Result handling

**Paragraphs.** Automatic mode uses reading order, local spacing, alignment, and reliable provider hints. Existing logical lines are preserved when coordinates are unavailable. Split mode retains OCR lines; merge mode explicitly removes all line breaks. Smart formatting is a separate action and does not change the default paragraph mode for later OCR requests.

**Tables.** Structure comes from provider JSON. Clipboard output preserves merged spans, cell line breaks, leading zeros, and long identifiers. Review recognition mistakes, missing cells, and incorrect merges. The left TSV pane scrolls horizontally; the right pane uses a native table viewer. Tables can be edited after pasting into Word / Excel.

**Formulas and Markdown.** Windows Rich Edit/TOM2 handles formulas, and pulldown-cmark parses Markdown. Common roots, fractions, scripts, headings, lists, and code are supported, with `$...$`, `$$...$$`, `\(...\)`, and `\[...\]` math delimiters. HTML is displayed literally; scripts are not executed and remote images are not loaded. Rendering is not a complete TeX or browser implementation, and complex macros depend on system capabilities. Previews larger than 256 KiB or 8192 formatting spans fall back to source.

**Images.** Manual scale ranges from 1–800%, with smooth convergence for continuous wheel input. Runtime-dispatched SIMD adaptive Bilinear filtering from `fast_image_resize` uses bounded tiles. Display buffers are separate from original pixels, so copying excludes borders, shadows, and information bars. Windows may move partly off-screen while retaining a reachable area.

## Data, credentials, and network use

- Default data directory: `%LOCALAPPDATA%\QingquOCR`. Preferences are stored in `settings.json`; `credentials.bin` is encrypted with Windows DPAPI. `QINGQU_DATA_DIR` selects an independent directory.
- The credential editor can show or hide saved keys. Do not commit credential files or screenshots containing keys.
- OCR sends the selected image to the bound provider; translation sends text to its configured provider. With automatic translation enabled, recognized text is subsequently sent for translation.
- Opening clipboard notes / images with `Alt+V` does not upload their content. Recognition and translation commands make requests according to your configuration.
- Original images and undo records use session temporary files. Original-image files are removed after the final window/task reference is released.
- Copying an image leaves a PNG in the system temporary directory `QingquOCR-clipboard` so its path survives application exit. Later successful copies remove files older than seven days and aim for a cache limit of 128 images / 128 MiB. Current or locked files may temporarily exceed those limits. Copied sensitive images can therefore remain after exit.

## Build from source

### Prerequisites

- Rust MSVC toolchain: `x86_64-pc-windows-msvc`. This version was checked with Rust 1.97.1; no minimum supported Rust version is declared.
- Visual Studio C++ Build Tools and Windows SDK. A C++17 compiler is required for the statically linked Scintilla sources.
- Network access for the first Cargo dependency fetch; offline builds are possible once dependencies are cached.
- Python 3.11+ for optional ZIP packaging only, not for running the application.

In PowerShell with the MSVC development environment configured:

```powershell
cargo build --release --locked
```

Output: `target/release/qingqu-ocr-native.exe`. `.cargo/config.toml` enables static CRT for Windows MSVC. Required Scintilla sources are included in `vendor/scintilla`.

Basic checks:

```powershell
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked -- -D warnings
```

Package the executable, READMEs, and notices:

```powershell
python scripts/package.py
```

The script reads the current executable and locked Windows dependency metadata, then creates `dist/qingqu-ocr-native-0.3.0-windows-x64.zip` and a SHA-256 file. Development notes, test reports, and historical experimental executables are excluded.

### Source layout

```text
src/                     Application, service adapters, and platform code
  config/                Task bindings, recognition, and language settings
  native/                Win32 windows, settings, editor, tables, and images
  ocr/                   Input budgets, formula, and spreadsheet handling
  platform/              Capture, clipboard, and local caches
  text/                  Formatting, reading order, and paragraph rules
assets/                  App icon, SVGs, themes, and manifest
vendor/                  Scintilla, image adapter, and algorithm attribution
licenses/                Third-party license texts and component inventory
docs/screenshots/        Consistently sized UI examples
scripts/package.py       Release packaging
LICENSE                  Noncommercial license for original project materials
NOTICE.txt               Copyright and license scope
THIRD_PARTY_NOTICES.txt   Third-party component information
```

## Known limitations

- Windows x64 only; no macOS / Linux build or bundled offline OCR engine.
- The application UI is primarily Chinese. An English README does not imply a fully localized English interface.
- Capture covers the monitor under the pointer and does not span multiple monitors. Captures exceeding 48 MiB of raw pixels are rejected.
- Input dimensions and PNG byte budgets are enforced before upload. Large images may be reduced, affecting small text. Automatic tiled recognition with additional requests is not implemented.
- Model availability, permissions, endpoint changes, and network conditions affect results. No fixed latency, free quota, or universal language accuracy is guaranteed.
- Verify custom, Qwen, and compatible protocols with your own account. Client support does not mean every model has been validated against a live cloud account.
- Touchpads, specific IMEs, mixed-DPI monitors, and complex formulas may behave differently across devices. Large-image frame rates and peak memory depend on CPU, viewport, and image dimensions.

## Feedback

Include the Windows and application versions, DPI, provider/model, reproduction steps, and expected behavior when reporting an issue. Attach a minimal nonsensitive sample and relevant screenshots. Do not attach API keys, `credentials.bin`, or private source documents.

## License

Original code, documentation, and original artwork that the project author has authority to license are available under **[PolyForm Noncommercial License 1.0.0](LICENSE)**, for the purposes that license permits. **Commercial use of those original materials is not granted.** This is a source-available noncommercial project; it does not claim an unrestricted open-source license.

Third-party components retain their own licenses. The project's noncommercial terms do not narrow rights granted by the original MIT, Apache-2.0, BSD, Unicode, Scintilla, or other applicable licenses. Umi-OCR / GapTree-derived layout portions retain their MIT attribution. Read [NOTICE.txt](NOTICE.txt), [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), and [licenses/](licenses/), and preserve applicable copyright, license, and notice files when distributing.

Thanks to the maintainers of Scintilla, Umi-OCR / GapTree, fast_image_resize, Microsoft Fluent UI System Icons, VS Code, Catppuccin, and the Rust ecosystem.
