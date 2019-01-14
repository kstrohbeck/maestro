use image::{self, jpeg::JPEGEncoder, png::PNGEncoder, DynamicImage, FilterType, Pixel, Rgb};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

pub enum Format {
    Png,
    Jpeg,
}

impl Format {
    fn as_ext(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
        }
    }
}

pub struct Image {
    data: Vec<u8>,
    pub format: Format,
}

impl Image {
    pub fn load<P>(path: P) -> Result<Self, ImageError>
    where
        P: AsRef<Path>,
    {
        let mut data = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        let format = match image::guess_format(&data[..])? {
            image::ImageFormat::PNG => Ok(Format::Png),
            image::ImageFormat::JPEG => Ok(Format::Jpeg),
            _ => Err(ImageError::UnsupportedFormat),
        }?;
        Ok(Self { data, format })
    }

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
        let images = images.as_ref();
        let cache = cache.as_ref();
        let fnames = ["png", "jpg", "jpeg"]
            .iter()
            .map(|ext| format!("{}.{}", name, ext))
            .collect::<Vec<_>>();

        for fname in &fnames {
            let path = cache.join(fname);
            if path.exists() {
                return Image::load(path);
            }
        }

        for fname in &fnames {
            let path = images.join(fname);
            if path.exists() {
                let raw = image::open(&path)?;
                let image = process(raw)?;
                let output_name = format!("{}.{}", name, image.format.as_ext());
                let cache_path = cache.join(output_name);
                File::create(cache_path)?.write_all(&image.data[..])?;
                return Ok(image);
            }
        }

        Err(ImageError::NoImage)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn as_dynamic(&self) -> image::ImageResult<DynamicImage> {
        let format = match self.format {
            Format::Png => image::ImageFormat::PNG,
            Format::Jpeg => image::ImageFormat::JPEG,
        };
        image::load_from_memory_with_format(self.data(), format)
    }
}

#[derive(Debug)]
pub enum ImageError {
    NoImage,
    Io(io::Error),
    Image(image::ImageError),
    UnsupportedFormat,
}

impl From<io::Error> for ImageError {
    fn from(err: io::Error) -> ImageError {
        ImageError::Io(err)
    }
}

impl From<image::ImageError> for ImageError {
    fn from(err: image::ImageError) -> ImageError {
        ImageError::Image(err)
    }
}

pub fn transform_image(img: DynamicImage) -> Result<Image, ImageError> {
    let img = img.resize(1000, 1000, FilterType::Lanczos3).to_rgb();

    // Try both PNG and JPEG encoding.
    let mut png_data = Vec::new();
    PNGEncoder::new(&mut png_data).encode(
        &img,
        img.width(),
        img.height(),
        <Rgb<u8> as Pixel>::color_type(),
    )?;

    let mut jpeg_data = Vec::new();
    JPEGEncoder::new(&mut jpeg_data).encode(
        &img,
        img.width(),
        img.height(),
        <Rgb<u8> as Pixel>::color_type(),
    )?;

    if png_data.len() <= jpeg_data.len() {
        Ok(Image {
            data: png_data,
            format: Format::Png,
        })
    } else {
        Ok(Image {
            data: jpeg_data,
            format: Format::Jpeg,
        })
    }
}

pub fn transform_image_vw(img: DynamicImage) -> Result<Image, ImageError> {
    let img = img.resize(300, 300, FilterType::Lanczos3).to_rgb();
    let mut data = Vec::new();
    JPEGEncoder::new(&mut data).encode(
        &img,
        img.width(),
        img.height(),
        <Rgb<u8> as Pixel>::color_type(),
    )?;
    Ok(Image {
        data,
        format: Format::Jpeg,
    })
}

#[cfg(test)]
mod tests {
    use super::{transform_image, transform_image_vw, Image, ImageError};
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
        let mut images = PathBuf::new();
        images.push(env!("CARGO_MANIFEST_DIR"));
        images.push("data");
        let cache = tempdir().ok().unwrap();
        let _ = Image::load_with_cache(&images, cache.path(), "coast", transform_image).unwrap();
        assert!(cache.path().join("coast.jpg").exists());
    }

    #[test]
    #[ignore]
    fn cached_image_is_used() {
        let mut images = PathBuf::new();
        images.push(env!("CARGO_MANIFEST_DIR"));
        images.push("data");
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
