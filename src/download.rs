use std::ffi::OsStr;

use axum::{
    body::StreamBody,
    extract::{Path, State},
    http::{header::CONTENT_TYPE, HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use eyre::{eyre, Result};
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
        "jpg" | "jpeg" => Ok((CONTENT_TYPE, "image/jpeg")),
        "png" => Ok((CONTENT_TYPE, "image/png")),
        "arw" => Ok((CONTENT_TYPE, "image/x-sony-arw")),
        _ => Ok((CONTENT_TYPE, "application/octet-stream")),
    }
}

pub async fn download(State(state): State<AppState>, Path(filename): Path<String>) -> Response {
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
