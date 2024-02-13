# Image Thumbs

Easy-to-use library to create image thumbnails from images existing on some (cloud) object storage or from disk.

Currently implemented is a connection to Google Cloud Storage, but it can be easily extended to other providers.

## Supported formats
PNG and JPEG are currently the only supported image formats.

# How to use
## Sizes
Configure what thumbnails you would like to have in a .yaml file:
```yaml
thumbs:
  - name: standard      # This name will be added to the thumbnail with an underscore (_)
    # Optional; The default pattern is /{image_stem}_{thumb_name}
    # The original extension is always appended to the end, e.g., `.png`
    naming_pattern: "/{thumb_name}/{image_stem}"
    quality: 80         # PNG ignores this variable as it is always lossless
    size: [ 640, 480 ]  # Target size of the thumbnail. May not always be exact.
    mode: fit           # Available are: 'fit' and 'crop'

  - name: mini
    quality: 80
    size: [ 40, 40 ]
    mode: crop
```

## Google credentials
This crate relies on [object_store](https://crates.io/crates/object_store) for the interaction with the storage backend.
Currently, this crate only supports Google Cloud Storage.

To configure the Google Service Account, use one of the following environment variables as 
[described in the object_store](https://docs.rs/object_store/0.9.0/object_store/gcp/struct.GoogleCloudStorageBuilder.html#method.from_env)
crate.

```text
GOOGLE_SERVICE_ACCOUNT: location of service account file
GOOGLE_SERVICE_ACCOUNT_PATH: (alias) location of service account file
SERVICE_ACCOUNT: (alias) location of service account file
GOOGLE_SERVICE_ACCOUNT_KEY: JSON serialized service account key
GOOGLE_BUCKET: bucket name
GOOGLE_BUCKET_NAME: (alias) bucket name
```

Then use it in your code 
```rust
#[tokio::main]
async fn main() {
    // Path to your thumbnail configuration yaml. You may specify the .yaml extension in the path, but you don't need to.
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