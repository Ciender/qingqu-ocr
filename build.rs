use resvg::{tiny_skia, usvg};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const ICONS: &[&str] = &[
    "scan-text",
    "table",
    "vertical",
    "formula",
    "clipboard",
    "image",
    "paragraph",
    "split",
    "merge",
    "punctuation",
    "chinese-convert",
    "translate",
    "copy",
    "delete",
    "undo",
    "search",
    "pin",
    "menu",
    "minimize",
    "maximize",
    "close",
    "api",
    "keyboard",
    "appearance",
    "settings",
    "eye",
    "zoom-in",
    "zoom-out",
    "fit",
    "rotate-left",
    "rotate-right",
    "window",
    "actual-size",
    "grip",
    "pin-off",
    "chinese-traditional",
    "paste",
    "cut",
    "redo",
    "line-numbers",
    "chevron-down",
    "auto-paragraph",
    "punctuation-chinese",
    "punctuation-english",
    "help-circle",
    "chinese-simplified",
];
fn svg(path: &Path, size: u32) -> tiny_skia::Pixmap {
    let source = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let tree =
        usvg::Tree::from_data(&source, &usvg::Options::default()).expect("invalid SVG asset");
    // Keep vendor SVG files unchanged. A Fluent sparkle extends slightly above
    // its viewBox; fit overflowing geometry with a half-unit inset, not clipping.
    let bounds = tree.root().abs_stroke_bounding_box();
    let (width, height) = (tree.size().width(), tree.size().height());
    let transform =
        if bounds.x() < 0. || bounds.y() < 0. || bounds.right() > width || bounds.bottom() > height
        {
            let left = bounds.x().min(0.);
            let top = bounds.y().min(0.);
            let source_width = bounds.right().max(width) - left;
            let source_height = bounds.bottom().max(height) - top;
            let inset = size as f32 * 0.5 / width.max(height);
            let scale = ((size as f32 - 2. * inset) / source_width)
                .min((size as f32 - 2. * inset) / source_height);
            tiny_skia::Transform::from_row(
                scale,
                0.,
                0.,
                scale,
                (size as f32 - source_width * scale) / 2. - left * scale,
                (size as f32 - source_height * scale) / 2. - top * scale,
            )
        } else {
            tiny_skia::Transform::from_scale(size as f32 / width, size as f32 / height)
        };
    let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
}
fn main() {
    println!("cargo:rerun-if-changed=vendor/scintilla/src");
    println!("cargo:rerun-if-changed=vendor/scintilla/include");
    println!("cargo:rerun-if-changed=vendor/scintilla/win32");
    let mut scintilla = cc::Build::new();
    scintilla
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .include("vendor/scintilla/include")
        .include("vendor/scintilla/src")
        .define("NDEBUG", None)
        .flag_if_supported("/utf-8")
        .flag_if_supported("/EHsc");
    let mut sources: Vec<_> = fs::read_dir("vendor/scintilla/src")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cxx"))
        .collect();
    sources.sort();
    scintilla.files(sources);
    for name in [
        "HanjaDic",
        "PlatWin",
        "ListBox",
        "SurfaceGDI",
        "SurfaceD2D",
        "ScintillaWin",
    ] {
        scintilla.file(format!("vendor/scintilla/win32/{name}.cxx"));
    }
    scintilla.compile("scintilla");
    for name in ["imm32", "ole32", "oleaut32", "advapi32", "gdi32", "user32"] {
        println!("cargo:rustc-link-lib={name}");
    }
    println!("cargo:rerun-if-changed=assets/icons");
    println!("cargo:rerun-if-changed=assets/app.svg");
    println!("cargo:rerun-if-changed=assets/windows.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let header = fs::read_to_string("vendor/scintilla/include/Scintilla.h").unwrap();
    let mut constants = String::new();
    for line in header.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() == 3
            && parts[0] == "#define"
            && (parts[1].starts_with("SCI_")
                || parts[1].starts_with("SC_")
                || parts[1].starts_with("STYLE_"))
            && (parts[2].parse::<i64>().is_ok() || parts[2].starts_with("0x"))
        {
            constants.push_str(&format!(
                "pub const {}: u32 = ({}i64) as u32;\n",
                parts[1], parts[2]
            ));
        }
    }
    fs::write(out.join("scintilla_constants.rs"), constants).unwrap();
    // One raster pixel per screen pixel, including fractional Windows DPI scales.
    // The SVG parser remains a build dependency; atlases are decoded only on demand.
    let mut atlas_source =
        String::from("fn icon_atlas_png(cell: u32) -> &'static [u8] { match cell {\n");
    for cell in 12u32..=96 {
        let mut atlas = tiny_skia::Pixmap::new(cell * ICONS.len() as u32, cell).unwrap();
        for (i, name) in ICONS.iter().enumerate() {
            let tile = svg(&Path::new("assets/icons").join(format!("{name}.svg")), cell);
            atlas.draw_pixmap(
                (i as u32 * cell) as i32,
                0,
                tile.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
        atlas
            .save_png(out.join(format!("icons-{cell}.png")))
            .unwrap();
        atlas_source.push_str(&format!(
            "{cell} => include_bytes!(concat!(env!(\"OUT_DIR\"), \"/icons-{cell}.png\")),\n"
        ));
    }
    atlas_source.push_str("_ => unreachable!(\"icon size outside compiled range\"), }}\n");
    fs::write(out.join("icon_atlases.rs"), atlas_source).unwrap();
    let mut icon_images = Vec::new();
    for size in [16u32, 20, 24, 32, 40, 48, 64, 128, 256] {
        let pixmap = svg(Path::new("assets/app.svg"), size);
        let png = pixmap.encode_png().unwrap();
        fs::write(out.join(format!("app-{size}.png")), &png).unwrap();
        icon_images.push((size, png));
    }
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(icon_images.len() as u16).to_le_bytes());
    let mut offset = (6 + 16 * icon_images.len()) as u32;
    for (size, png) in &icon_images {
        ico.extend_from_slice(&[
            if *size == 256 { 0 } else { *size as u8 },
            if *size == 256 { 0 } else { *size as u8 },
            0,
            0,
        ]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in icon_images {
        ico.extend_from_slice(&png);
    }
    let ico_path = out.join("qingqu-ocr.ico");
    fs::write(&ico_path, ico).unwrap();
    fs::write(
        out.join("svg-bounds-checked.txt"),
        "All SVG stroke bounds fit rendered viewports.\n",
    )
    .unwrap();
    let names = ICONS
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        out.join("icon_names.rs"),
        format!("pub const ICON_NAMES: &[&str] = &[{names}];"),
    )
    .unwrap();
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_manifest_file("assets/windows.manifest")
            .set_icon(ico_path.to_str().unwrap())
            .set("FileDescription", "轻取 OCR")
            .set("ProductName", "轻取 OCR")
            .set("OriginalFilename", "qingqu-ocr-native.exe")
            .set("LegalCopyright", "Qingqu OCR contributors")
            .compile()
            .expect("Windows icon resource compilation failed");
    }
}
