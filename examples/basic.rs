use std::path::Path;

#[tokio::main]
async fn main() {
    let thumbs = image_thumbs::ImageThumbs::new("examples/image_thumbs").await.unwrap();
    dbg!(&thumbs);
    let hi = thumbs.create_thumbs(Path::new("test.jpg")).await;
    dbg!(hi);
}