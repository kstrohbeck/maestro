//! Image handling and transformation.

use image::DynamicImage;
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    fmt, fs,
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
    /// # use maestro::image::Format;
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
    type Error = FormatError;

    fn try_from(format: image::ImageFormat) -> Result<Self, Self::Error> {
        match format {
            image::ImageFormat::Png => Ok(Format::Png),
            image::ImageFormat::Jpeg => Ok(Format::Jpeg),
            _ => Err(FormatError { format }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatError {
    format: image::ImageFormat,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid format: {:?}", self.format)
    }
}

impl Error for FormatError {}

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
    /// # use maestro::image::{Image, LoadError};
    /// let img = Image::load("images/foo.jpg")?;
    /// # Ok::<(), LoadError>(())
    /// ```
    pub fn load<P>(path: P) -> Result<Self, LoadError>
    where
        P: AsRef<Path>,
    {
        let data = fs::read(path).map_err(LoadError::CouldntReadFile)?;
        let format = image::guess_format(&data[..])
            .map_err(LoadError::CouldntDetectFormat)?
            .try_into()
            .map_err(LoadError::UnsupportedFormat)?;
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
    /// # use maestro::image::{Image, LoadWithCacheError, transform_image};
    /// let img = Image::load_with_cache("images", ".cache", "foo", transform_image)?;
    /// # Ok::<(), LoadWithCacheError>(())
    /// ```
    pub fn load_with_cache<P, Q, F>(
        images: P,
        cache: Q,
        name: &str,
        process: F,
    ) -> Result<Self, LoadWithCacheError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        F: Fn(DynamicImage) -> Result<Self, image::ImageError>,
    {
        let images = images.as_ref();
        let cache = cache.as_ref();
        let fnames = ["png", "jpg", "jpeg"]
            .iter()
            .map(|ext| format!("{}.{}", name, ext))
            .collect::<Vec<_>>();

        let mut images_paths = fnames.iter().map(|n| images.join(n));
        let mut cache_paths = fnames.iter().map(|n| cache.join(n));

        if let Some(path) = cache_paths.find(|p| p.exists()) {
            Image::load(path).map_err(LoadWithCacheError::CacheLoadError)
        } else if let Some(path) = images_paths.find(|p| p.exists()) {
            let raw = image::open(&path).map_err(LoadWithCacheError::CouldntOpenUncachedImage)?;
            let image = process(raw).map_err(LoadWithCacheError::ProcessError)?;
            // Ensure that the cache folder exists.
            fs::create_dir_all(cache).map_err(LoadWithCacheError::CouldntCreateCacheFolder)?;
            let output_name = format!("{}.{}", name, image.format.ext());
            let cache_path = cache.join(output_name);
            fs::write(cache_path, &image.data[..])
                .map_err(LoadWithCacheError::CouldntWriteCachedFile)?;
            Ok(image)
        } else {
            Err(LoadWithCacheError::NoImage)
        }
    }

    /// Optionally load an image at a path.
    ///
    /// If no image exists cached or uncached, this returns an `Ok` containing a `None`.
    /// If any other errors occur, it returns an `Err`.
    pub fn try_load_with_cache<P, Q, F>(
        images: P,
        cache: Q,
        name: &str,
        process: F,
    ) -> Result<Option<Self>, LoadWithCacheError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        F: Fn(DynamicImage) -> Result<Self, image::ImageError>,
    {
        match Self::load_with_cache(images, cache, name, process) {
            Ok(img) => Ok(Some(img)),
            Err(LoadWithCacheError::NoImage) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get the image data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Create a savable image from the data.
    pub fn as_dynamic(&self) -> image::ImageResult<DynamicImage> {
        let format = match self.format {
            Format::Png => image::ImageFormat::Png,
            Format::Jpeg => image::ImageFormat::Jpeg,
        };
        image::load_from_memory_with_format(self.data(), format)
    }
}

/// An error when loading an image.
#[derive(Debug)]
pub enum LoadError {
    NoImage,
    CouldntReadFile(std::io::Error),
    CouldntDetectFormat(image::ImageError),
    UnsupportedFormat(FormatError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadError::NoImage => write!(f, "no image found"),
            LoadError::CouldntReadFile(e) => write!(f, "couldn't read file: {}", e),
            LoadError::CouldntDetectFormat(e) => write!(f, "couldn't detect format: {}", e),
            LoadError::UnsupportedFormat(e) => write!(f, "unsupported format: {}", e),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadError::NoImage => None,
            LoadError::CouldntReadFile(e) => Some(e),
            LoadError::CouldntDetectFormat(e) => Some(e),
            LoadError::UnsupportedFormat(e) => Some(e),
        }
    }
}

/// An error when loading an image with a cache backup.
#[derive(Debug)]
pub enum LoadWithCacheError {
    NoImage,
    CacheLoadError(LoadError),
    CouldntOpenUncachedImage(image::ImageError),
    ProcessError(image::ImageError),
    CouldntCreateCacheFolder(std::io::Error),
    CouldntWriteCachedFile(std::io::Error),
}

impl fmt::Display for LoadWithCacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadWithCacheError::NoImage => write!(f, "no image found"),
            LoadWithCacheError::CacheLoadError(e) => write!(f, "error with cache file: {}", e),
            LoadWithCacheError::CouldntOpenUncachedImage(e) => {
                write!(f, "couldn't open uncached image: {}", e)
            }
            LoadWithCacheError::ProcessError(e) => write!(f, "error processing image: {}", e),
            LoadWithCacheError::CouldntCreateCacheFolder(e) => {
                write!(f, "couldn't create cache folder: {}", e)
            }
            LoadWithCacheError::CouldntWriteCachedFile(e) => {
                write!(f, "couldn't write cached file: {}", e)
            }
        }
    }
}

impl Error for LoadWithCacheError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadWithCacheError::NoImage => None,
            LoadWithCacheError::CacheLoadError(e) => Some(e),
            LoadWithCacheError::CouldntOpenUncachedImage(e)
            | LoadWithCacheError::ProcessError(e) => Some(e),
            LoadWithCacheError::CouldntCreateCacheFolder(e)
            | LoadWithCacheError::CouldntWriteCachedFile(e) => Some(e),
        }
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
                <image::Rgb<u8> as image::Pixel>::COLOR_TYPE,
            )
            .map(|()| data)
    }};
}

/// Transform an image into a standard format.
///
/// The transformed image is 1000x1000 pixels, and may be a PNG or JPEG. The encoding used is
/// whichever produces a smaller-sized output.
pub fn transform_image(img: DynamicImage) -> Result<Image, image::ImageError> {
    use image::{jpeg::JpegEncoder, png::PngEncoder};

    let img = img
        .resize(1000, 1000, image::imageops::FilterType::Lanczos3)
        .to_rgb8();

    // Try both PNG and JPEG encoding.
    let png_data = encode!(PngEncoder, &img)?;
    let jpeg_data = encode!(JpegEncoder, &img)?;

    Ok(if png_data.len() <= jpeg_data.len() {
        Image::from_png(png_data)
    } else {
        Image::from_jpeg(jpeg_data)
    })
}

/// Transform an image into a format for car use.
pub fn transform_image_vw(img: DynamicImage) -> Result<Image, image::ImageError> {
    use image::jpeg::JpegEncoder;

    let img = img
        .resize(300, 300, image::imageops::FilterType::Lanczos3)
        .to_rgb8();
    let data = encode!(JpegEncoder, &img)?;
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
