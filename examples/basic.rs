use image_thumbs::GoogleCloudStorage;

#[tokio::main]
async fn main() {
    let thumbs =
        image_thumbs::ImageThumbs::<GoogleCloudStorage>::new("examples/image_thumbs").unwrap();
    thumbs
        .create_thumbs("penguin.jpg", "/thumbs", false)
        .await
        .unwrap();
    thumbs
        .create_thumbs("penguin.png", "/thumbs", false)
        .await
        .unwrap();
}
