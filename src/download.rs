use std::ffi::OsStr;

use axum::{
    body::StreamBody,
    extract::{Path, State},
    http::{header::CONTENT_TYPE, HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use eyre::Result;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{jpg, AppState};

/// Images are named <CHECKSUM>.<EXTENSION>. Pass in the filename of the _not optimized_ image.
/// This function will return the path to the optimized image. It may need to optimize it if it
/// does not exist.
fn get_optimized(state: AppState, name: &str) -> Result<std::path::PathBuf> {
    let mut path = state.optimized();
    path.push(name);

    // No optimized image? Do that.
    // It will be fairly slow, but should only need doing once. Unless the cache expires or is
    // cleared.
    if !path.exists() {
        let mut input = state.images();
        input.push(name);

        // TODO: support other filetypes
        match input.extension().and_then(OsStr::to_str) {
            Some("jpg") | Some("jpeg") => jpg::optimize(&input, &path)?,
            _ => {
                // If we can't optimize an image, just return the same path we get
                let mut path = state.images();
                path.push(name);
                return Ok(path);
            }
        };
    }

    Ok(path)
}

/// Either the file is cached so we can immediately return it, or it isn't.
/// If not, download to the cache, optimize, then return it.
pub async fn cache_dir_or_s3(state: AppState, filename: &str) -> Result<File> {
    // For now it just mirrors the bucket structure.
    // In the future I'll probably want to do something that's more friendly to
    // Linux filesystem nodes, vs S3 magic
    let mut path = state.images();
    path.push(filename);

    // First, check if our local cache has the image. If not, download from S3.
    if !path.exists() {
        let mut output = tokio::fs::File::create(path.clone()).await?;

        state
            .bucket
            .get_object_to_writer(format!("images/{}", filename), &mut output)
            .await?;
    }

    // Then get the optimized version
    let optimized = get_optimized(state, filename)?;

    let file = tokio::fs::File::open(optimized).await?;
    Ok(file)
}

/// Take a file extension and return a content type header
fn extension_to_header(extension: &str) -> Result<(HeaderName, &'static str)> {
    match extension.to_lowercase().as_str() {
        // Images
        "jpg" | "jpeg" => Ok((CONTENT_TYPE, "image/jpeg")),
        "png" => Ok((CONTENT_TYPE, "image/png")),
        "gif" => Ok((CONTENT_TYPE, "image/gif")),
        "webp" => Ok((CONTENT_TYPE, "image/webp")),
        "svg" => Ok((CONTENT_TYPE, "image/svg+xml")),
        "ico" => Ok((CONTENT_TYPE, "image/x-icon")),
        "tiff" | "tif" => Ok((CONTENT_TYPE, "image/tiff")),
        "bmp" => Ok((CONTENT_TYPE, "image/bmp")),
        "arw" => Ok((CONTENT_TYPE, "image/x-sony-arw")),

        // Audio
        "mp3" => Ok((CONTENT_TYPE, "audio/mpeg")),
        "wav" => Ok((CONTENT_TYPE, "audio/wav")),
        "ogg" => Ok((CONTENT_TYPE, "audio/ogg")),
        "flac" => Ok((CONTENT_TYPE, "audio/flac")),
        "aac" => Ok((CONTENT_TYPE, "audio/aac")),

        // Video
        "mp4" => Ok((CONTENT_TYPE, "video/mp4")),
        "webm" => Ok((CONTENT_TYPE, "video/webm")),
        "avi" => Ok((CONTENT_TYPE, "video/x-msvideo")),
        "mov" => Ok((CONTENT_TYPE, "video/quicktime")),
        "wmv" => Ok((CONTENT_TYPE, "video/x-ms-wmv")),

        // Documents
        "pdf" => Ok((CONTENT_TYPE, "application/pdf")),
        "doc" | "docx" => Ok((CONTENT_TYPE, "application/msword")),
        "xls" | "xlsx" => Ok((CONTENT_TYPE, "application/vnd.ms-excel")),
        "ppt" | "pptx" => Ok((CONTENT_TYPE, "application/vnd.ms-powerpoint")),
        "txt" => Ok((CONTENT_TYPE, "text/plain")),
        "rtf" => Ok((CONTENT_TYPE, "application/rtf")),
        "csv" => Ok((CONTENT_TYPE, "text/csv")),

        // Web
        "html" | "htm" => Ok((CONTENT_TYPE, "text/html")),
        "css" => Ok((CONTENT_TYPE, "text/css")),
        "js" => Ok((CONTENT_TYPE, "application/javascript")),
        "json" => Ok((CONTENT_TYPE, "application/json")),
        "xml" => Ok((CONTENT_TYPE, "application/xml")),

        // Archives
        "zip" => Ok((CONTENT_TYPE, "application/zip")),
        "rar" => Ok((CONTENT_TYPE, "application/vnd.rar")),
        "7z" => Ok((CONTENT_TYPE, "application/x-7z-compressed")),
        "tar" => Ok((CONTENT_TYPE, "application/x-tar")),
        "gz" => Ok((CONTENT_TYPE, "application/gzip")),

        // Fonts
        "ttf" => Ok((CONTENT_TYPE, "font/ttf")),
        "otf" => Ok((CONTENT_TYPE, "font/otf")),
        "woff" => Ok((CONTENT_TYPE, "font/woff")),
        "woff2" => Ok((CONTENT_TYPE, "font/woff2")),

        // Code file types
        "py" => Ok((CONTENT_TYPE, "text/x-python")),
        "java" => Ok((CONTENT_TYPE, "text/x-java-source")),
        "c" => Ok((CONTENT_TYPE, "text/x-c")),
        "cpp" | "cxx" | "cc" => Ok((CONTENT_TYPE, "text/x-c++")),
        "h" | "hpp" => Ok((CONTENT_TYPE, "text/x-c++hdr")),
        "rs" => Ok((CONTENT_TYPE, "text/x-rust")),
        "go" => Ok((CONTENT_TYPE, "text/x-go")),
        "rb" => Ok((CONTENT_TYPE, "text/x-ruby")),
        "php" => Ok((CONTENT_TYPE, "application/x-httpd-php")),
        "swift" => Ok((CONTENT_TYPE, "text/x-swift")),
        "kt" | "kts" => Ok((CONTENT_TYPE, "text/x-kotlin")),
        "scala" => Ok((CONTENT_TYPE, "text/x-scala")),
        "pl" | "pm" => Ok((CONTENT_TYPE, "text/x-perl")),
        "sh" => Ok((CONTENT_TYPE, "application/x-sh")),
        "ts" => Ok((CONTENT_TYPE, "application/typescript")),
        "jsx" | "tsx" => Ok((CONTENT_TYPE, "text/jsx")),
        "vue" => Ok((CONTENT_TYPE, "text/x-vue")),
        "dart" => Ok((CONTENT_TYPE, "application/vnd.dart")),
        "sql" => Ok((CONTENT_TYPE, "application/sql")),
        "lua" => Ok((CONTENT_TYPE, "text/x-lua")),
        "r" => Ok((CONTENT_TYPE, "text/x-r")),
        "m" => Ok((CONTENT_TYPE, "text/x-objectivec")),

        // Default case for unknown extensions
        _ => Ok((CONTENT_TYPE, "application/octet-stream")),
    }
}

pub async fn download(State(state): State<AppState>, Path(filename): Path<String>) -> Response {
    let filename = sanitize_filename(filename.as_str());
    let file = cache_dir_or_s3(state, filename.as_str()).await;

    let file = match file {
        Ok(stream) => stream,
        Err(e) => {
            println!("something broke: {}", e);

            return (StatusCode::NOT_FOUND, "Not found.").into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let filename = std::path::PathBuf::from(filename);
    let extension = filename.extension().and_then(OsStr::to_str);
    let content_type = extension_to_header(extension.unwrap_or("not_valid"));

    match content_type {
        Ok(headers) => ([headers], body).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not process filetype",
        )
            .into_response(),
    }
}

fn sanitize_filename(input: &str) -> String {
    // Remove any directory traversal attempts
    let filename = input.split(['/', '\\'].as_ref()).last().unwrap_or("");

    // Replace or remove potentially dangerous characters
    filename
        .chars()
        .filter_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' => Some(c),
            _ => None,
        })
        .collect::<String>()
        .trim_start_matches('.')
        .to_string()
}
