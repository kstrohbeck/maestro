//! Image handling and transformation.

use image::DynamicImage;
use std::{
    convert::{TryFrom, TryInto},
    path::Path,
};

/// An image format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
}

impl Format {
    /// Gets the format's file extension.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use songmaster_rs::image::Format;
    /// let img_fmt = Format::Png;
    /// let filename = format!("foo.{}", img_fmt.ext());
    /// ```
    pub fn ext(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
        }
    }

    /// Gets the format's MIME type.
    pub fn mime(self) -> &'static str {
        match self {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
        }
    }
}

impl TryFrom<image::ImageFormat> for Format {
    type Error = ImageError;

    fn try_from(value: image::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image::ImageFormat::PNG => Ok(Format::Png),
            image::ImageFormat::JPEG => Ok(Format::Jpeg),
            _ => Err(ImageError::UnsupportedFormat),
        }
    }
}

/// Raw image data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub data: Vec<u8>,
    pub format: Format,
}

impl Image {
    /// Create a new image from data.
    pub fn new(data: Vec<u8>, format: Format) -> Self {
        Self { data, format }
    }

    /// Create an `Image` from PNG data.
    pub fn from_png(data: Vec<u8>) -> Self {
        Self::new(data, Format::Png)
    }

    /// Create an `Image` from JPEG data.
    pub fn from_jpeg(data: Vec<u8>) -> Self {
        Self::new(data, Format::Jpeg)
    }

    /// Load an image at a path.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use songmaster_rs::image::{Image, ImageError};
    /// let img = Image::load("images/foo.jpg")?;
    /// # Ok::<(), ImageError>(())
    /// ```
    pub fn load<P>(path: P) -> Result<Self, ImageError>
    where
        P: AsRef<Path>,
    {
        use std::fs;

        let data = fs::read(path)?;
        let format = image::guess_format(&data[..])?.try_into()?;
        Ok(Self { data, format })
    }

    /// Load an image at a path, taking a cached version if it exists.
    ///
    /// This function searches for images with the `.png`, `.jpg`, and `.jpeg` file extensions,
    /// processes their raw data using `process`, and returns the resultant image. It checks for
    /// pre-processed images in the cache first. If it finds an image that was not in the cache, it
    /// caches the processed image.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use songmaster_rs::image::{Image, ImageError, transform_image};
    /// let img = Image::load_with_cache("images", ".cache", "foo", transform_image)?;
    /// # Ok::<(), ImageError>(())
    /// ```
    pub fn load_with_cache<P, Q, F>(
        images: P,
        cache: Q,
        name: &str,
        process: F,
    ) -> Result<Self, ImageError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        F: Fn(DynamicImage) -> Result<Self, ImageError>,
    {
        use std::fs;

        let images = images.as_ref();
        let cache = cache.as_ref();
        let fnames = ["png", "jpg", "jpeg"]
            .iter()
            .map(|ext| format!("{}.{}", name, ext))
            .collect::<Vec<_>>();

        let mut images_paths = fnames.iter().map(|n| images.join(n));
        let mut cache_paths = fnames.iter().map(|n| cache.join(n));

        if let Some(path) = cache_paths.find(|p| p.exists()) {
            Image::load(path)
        } else if let Some(path) = images_paths.find(|p| p.exists()) {
            let raw = image::open(&path)?;
            let image = process(raw)?;
            let output_name = format!("{}.{}", name, image.format.ext());
            let cache_path = cache.join(output_name);
            fs::write(cache_path, &image.data[..])?;
            Ok(image)
        } else {
            Err(ImageError::NoImage)
        }
    }

    /// Get the image data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Create a savable image from the data.
    pub fn as_dynamic(&self) -> image::ImageResult<DynamicImage> {
        let format = match self.format {
            Format::Png => image::ImageFormat::PNG,
            Format::Jpeg => image::ImageFormat::JPEG,
        };
        image::load_from_memory_with_format(self.data(), format)
    }
}

/// An error when loading or transforming an image.
#[derive(Debug)]
pub enum ImageError {
    NoImage,
    Io(std::io::Error),
    Image(image::ImageError),
    UnsupportedFormat,
}

impl From<std::io::Error> for ImageError {
    fn from(err: std::io::Error) -> ImageError {
        ImageError::Io(err)
    }
}

impl From<image::ImageError> for ImageError {
    fn from(err: image::ImageError) -> ImageError {
        ImageError::Image(err)
    }
}

macro_rules! encode {
    ( $enc:ident, $img:expr ) => {{
        let mut data = Vec::new();
        $enc::new(&mut data)
            .encode(
                $img,
                $img.width(),
                $img.height(),
                <image::Rgb<u8> as image::Pixel>::color_type(),
            )
            .map(|()| data)
    }};
}

/// Transform an image into a standard format.
///
/// The transformed image is 1000x1000 pixels, and may be a PNG or JPEG. The encoding used is
/// whichever produces a smaller-sized output.
pub fn transform_image(img: DynamicImage) -> Result<Image, ImageError> {
    use image::{jpeg::JPEGEncoder, png::PNGEncoder};

    let img = img.resize(1000, 1000, image::FilterType::Lanczos3).to_rgb();

    // Try both PNG and JPEG encoding.
    let png_data = encode!(PNGEncoder, &img)?;
    let jpeg_data = encode!(JPEGEncoder, &img)?;

    if png_data.len() <= jpeg_data.len() {
        Ok(Image::from_png(png_data))
    } else {
        Ok(Image::from_jpeg(jpeg_data))
    }
}

/// Transform an image into a format for car use.
pub fn transform_image_vw(img: DynamicImage) -> Result<Image, ImageError> {
    use image::jpeg::JPEGEncoder;

    let img = img.resize(300, 300, image::FilterType::Lanczos3).to_rgb();
    let data = encode!(JPEGEncoder, &img)?;
    Ok(Image::from_jpeg(data))
}

#[cfg(test)]
mod tests {
    use super::{transform_image, transform_image_vw, Image};
    use image::{self, DynamicImage, GenericImageView};
    use std::{
        fs::{self, File},
        io::Read,
        path::PathBuf,
    };
    use tempfile::tempdir;

    #[test]
    #[ignore]
    fn transformed_uncached_image_is_saved_in_cache() {
        let images = [env!("CARGO_MANIFEST_DIR"), "data"]
            .iter()
            .collect::<PathBuf>();
        let cache = tempdir().ok().unwrap();
        let _ = Image::load_with_cache(&images, cache.path(), "coast", transform_image).unwrap();
        assert!(cache.path().join("coast.jpg").exists());
    }

    #[test]
    #[ignore]
    fn cached_image_is_used() {
        let images = [env!("CARGO_MANIFEST_DIR"), "data"]
            .iter()
            .collect::<PathBuf>();
        let uncached_img = images.join("coast.jpg");
        let cache = tempdir().ok().unwrap();
        let cached_img = cache.path().join("coast.jpg");
        fs::copy(&uncached_img, &cached_img).unwrap();
        let img = Image::load_with_cache(&images, cache.path(), "coast", transform_image).unwrap();
        let mut cached = Vec::new();
        File::open(&cached_img)
            .unwrap()
            .read_to_end(&mut cached)
            .unwrap();
        assert_eq!(&img.data[..], &cached[..]);
    }

    #[test]
    #[ignore]
    fn transform_image_upsizes_to_1000_px_image() {
        let img = DynamicImage::new_rgba8(500, 700);
        let new_img = transform_image(img).ok().unwrap().as_dynamic().unwrap();
        assert_eq!(new_img.height(), 1000);
    }

    #[test]
    #[ignore]
    fn transform_image_downsizes_to_1000_px_image() {
        let img = DynamicImage::new_rgba8(1200, 1100);
        let new_img = transform_image(img).ok().unwrap().as_dynamic().unwrap();
        assert_eq!(new_img.width(), 1000);
    }

    #[test]
    #[ignore]
    fn transform_image_vw_upsizes_to_300_px_image() {
        let img = DynamicImage::new_rgba8(200, 250);
        let new_img = transform_image_vw(img).ok().unwrap().as_dynamic().unwrap();
        assert_eq!(new_img.height(), 300);
    }

    #[test]
    #[ignore]
    fn transform_image_vw_downsizes_to_300_px_image() {
        let img = DynamicImage::new_rgba8(600, 500);
        let new_img = transform_image_vw(img).ok().unwrap().as_dynamic().unwrap();
        assert_eq!(new_img.width(), 300);
    }
}
