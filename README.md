# Image Thumbs

Easy-to-use library to create image thumbnails from images existing on some (cloud) object storage or from disk.

Currently implemented is a connection to Google Cloud Storage, but it can be easily extended to other providers.

# How to use

 ```rust
#[tokio::main]
async fn main() {
    let thumbs = image_thumbs::ImageThumbs::new("examples/image_thumbs")
        .await
        .unwrap();
    thumbs
        .create_thumbs("penguin.jpg", "/thumbs", false)
        .await
        .unwrap();
    thumbs
        .create_thumbs("penguin.png", "/thumbs", false)
        .await
        .unwrap();
}
 ```