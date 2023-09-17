use std::path::PathBuf;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use s3::{creds::Credentials, Bucket};

mod download;
mod exif;
mod jpg;
mod upload;

// 100mb. Some camera JPEGs can be really big!
const MAX_IMAGE_SIZE: usize = 100 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    // that holds some api specific state
    bucket: Bucket,
    secret: String,
    cache: String,
    url: String,
}

impl AppState {
    pub fn images(&self) -> PathBuf {
        let mut path = PathBuf::from(&self.cache);
        path.push("images");

        path
    }

    pub fn optimized(&self) -> PathBuf {
        let mut path = PathBuf::from(&self.cache);
        path.push("optimized");

        path
    }
}

#[tokio::main]
async fn main() {
    // This is single user, so just expect the user to set the auth key. It'll do!
    let secret = std::env::var("API_SECRET").expect("Please set API_SECRET");
    let s3_region = std::env::var("S3_REGION").expect("Please set S3_REGION");
    let s3_bucket = std::env::var("S3_BUCKET").expect("Please set S3_BUCKET");
    let url = std::env::var("URL").expect("Please set URL");
    let cache_path = std::env::var("CACHE_PATH").unwrap_or(String::from("./cache"));

    let bucket = Bucket::new(
        s3_bucket.as_str(),
        s3_region.parse().expect("Failed to parse S3_REGION"),
        // Credentials are collected from environment, config, profile or instance metadata
        Credentials::default().expect("Please configure AWS credentials"),
    )
    .expect("Failed to create bucket");

    let state = AppState {
        bucket,
        secret,
        cache: cache_path,
        url,
    };

    _ = std::fs::create_dir_all(state.images());
    _ = std::fs::create_dir_all(state.optimized());

    let app = Router::new()
        .route("/upload", post(upload::upload))
        .route("/i/:filename", get(download::download))
        .layer(DefaultBodyLimit::max(MAX_IMAGE_SIZE))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
