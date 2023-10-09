#[tokio::main]
async fn main() {
    let thumbs = image_thumbs::ImageThumbs::new("examples/image_thumbs")
        .await
        .unwrap();
    thumbs.create_thumbs("penguin.jpg").await.unwrap();
    thumbs.create_thumbs("penguin.png").await.unwrap();
}
