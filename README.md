# Image Thumbs

Easy-to-use library to create image thumbnails from images existing on some (cloud) object storage or from disk.

Currently implemented is a connection to Google Cloud Storage, but it can be easily extended to other providers.

# How to use
Configure what thumbnails you would like to have:
```yaml
thumbs:
  - name: standard      # this name will be added to the thumbnail with an underscore (_)
    quality: 80         # PNG ignores this variable as it is always lossless
    size: [ 640, 480 ]  # Target size of the thumbnail. May not always be exact.
    mode: fit           # available are: 'fit' and 'crop'

  - name: mini
    quality: 80
    size: [ 40, 40 ]
    mode: crop
```

Then use it in your code 
```rust
#[tokio::main]
async fn main() {
    let thumbs = image_thumbs::ImageThumbs::new("examples/image_thumbs")
        .await
        .unwrap();
    thumbs
        .create_thumbs("penguin.jpg", "/thumbs", false)  // do not override existing images
        .await
        .unwrap();
    thumbs
        .create_thumbs("penguin.png", "/thumbs", true)  // do override existing images
        .await
        .unwrap();
}
```