use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "ui/dist/"]
struct EmbeddedAssets;

pub struct AssetFile {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

pub fn load_asset(path: &str) -> Option<AssetFile> {
    let normalized = normalize_path(path);
    if cfg!(debug_assertions)
        && let Some(asset) = load_from_filesystem(&normalized)
    {
        return Some(asset);
    }

    EmbeddedAssets::get(&normalized).map(|file| AssetFile {
        bytes: match file.data {
            Cow::Borrowed(bytes) => bytes.to_vec(),
            Cow::Owned(bytes) => bytes,
        },
        content_type: guess_content_type(&normalized),
    })
}

fn load_from_filesystem(path: &str) -> Option<AssetFile> {
    let direct = PathBuf::from("ui").join("dist").join(path);
    if let Ok(bytes) = fs::read(&direct) {
        return Some(AssetFile {
            bytes,
            content_type: guess_content_type(path),
        });
    }

    if let Some(candidate) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("ui").join("dist").join(path)))
        && let Ok(bytes) = fs::read(candidate)
    {
        return Some(AssetFile {
            bytes,
            content_type: guess_content_type(path),
        });
    }
    None
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        "index.html".to_string()
    } else {
        trimmed.to_string()
    }
}

fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}
