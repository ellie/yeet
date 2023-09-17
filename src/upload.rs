use std::ffi::OsStr;

use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use eyre::Result;
use s3::Bucket;

use crate::AppState;

// Upload the file, returning the name we generated for it.
// foo.jpg -> blake3_hash_of_foo.jpg
async fn upload_file(bucket: &Bucket, filename: String, file: Bytes) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(file.as_ref());
    let hash = hasher.finalize();
    let hash = hash.to_hex().to_string();
    println!("filename: {}", filename);
    let extension = std::path::Path::new(&filename)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap();

    let path = format!("images/{}.{}", hash, extension);

    let resp = bucket.put_object(path.clone(), file.as_ref()).await?;

    println!("got {} uploading image", resp.status_code());

    Ok(format!("{hash}.{extension}"))
}

// TODO: Instead of just loading the whole thing into memory, I can
// do so one chunk at a time + use s3 multipart
// I'm getting away with being lazy because I'll only really upload one
// image at a time (this doesn't need to scale)
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let auth = headers
        .get("Authorization")
        .and_then(|auth| auth.to_str().ok());

    if let Some(auth) = auth {
        if auth != state.secret {
            return (StatusCode::UNAUTHORIZED, "Invalid auth").into_response();
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Invalid auth").into_response();
    }

    if let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        let filename = upload_file(&state.bucket, name, data).await;

        if let Err(err) = filename {
            println!("An error occured: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to upload").into_response();
        }

        let full_url = format!("{}/i/{}", state.url, filename.unwrap());
        return full_url.into_response();
    }

    "OK".into_response()
}
