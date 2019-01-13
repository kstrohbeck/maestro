use crate::{
    models::{album::Album, disc::DiscInContext},
    text::Text,
    utils::{comma_separated, num_digits},
};
use image::{self, jpeg::JPEGEncoder, FilterType, ImageError, ImageFormat, Pixel, Rgb};
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

pub struct Track {
    title: Text,
    artists: Option<Vec<Text>>,
    year: Option<usize>,
    genre: Option<Text>,
    pub comment: Option<Text>,
    pub lyrics: Option<Text>,
}

impl Track {
    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn artists(&self) -> Option<&[Text]> {
        self.artists.as_ref().map(Vec::as_slice)
    }
}

pub struct TrackInContext<'a> {
    pub disc: &'a DiscInContext<'a>,
    track: &'a Track,
    pub track_number: usize,
}

impl<'a> TrackInContext<'a> {
    pub fn new(
        disc: &'a DiscInContext<'a>,
        track: &'a Track,
        track_number: usize,
    ) -> TrackInContext<'a> {
        TrackInContext {
            disc,
            track,
            track_number,
        }
    }

    fn album(&self) -> &Album {
        self.disc.album
    }

    pub fn title(&self) -> &Text {
        self.track.title()
    }

    pub fn artists(&self) -> &[Text] {
        self.track
            .artists()
            .unwrap_or_else(|| self.disc.album.artists())
    }

    pub fn artist(&self) -> Text {
        comma_separated(self.artists())
    }

    pub fn album_artists(&self) -> Option<&[Text]> {
        if self.artists() != self.album().artists() {
            Some(self.album().artists())
        } else {
            None
        }
    }

    pub fn album_artist(&self) -> Option<Text> {
        self.album_artists().map(comma_separated)
    }

    pub fn year(&self) -> Option<usize> {
        self.track.year.or(self.album().year)
    }

    pub fn genre(&self) -> Option<&Text> {
        self.track.genre.as_ref().or_else(|| self.album().genre())
    }

    pub fn comment(&self) -> Option<&Text> {
        self.track.comment.as_ref()
    }

    pub fn lyrics(&self) -> Option<&Text> {
        self.track.lyrics.as_ref()
    }

    pub fn filename(&self) -> String {
        let digits = num_digits(self.disc.num_tracks());
        format!(
            "{:0width$} - {}.mp3",
            self.track_number,
            self.title().file_safe(),
            width = digits
        )
    }

    pub fn path(&self) -> PathBuf {
        self.disc.path().join(self.filename())
    }

    pub fn cover_path(&self) -> Option<PathBuf> {
        for ext in &["png", "jpg", "jpeg"] {
            let fname = format!("{}.{}", self.track.title().file_safe(), ext);
            let path = self.album().cache_path().join(fname);
            if path.exists() {
                return Some(path);
            }
        }
        return self.album().cover_path();
    }

    pub fn cover(&self) -> Result<Cover, CoverError> {
        let path = self.cover_path().ok_or(CoverError::NoCover)?;
        let mut image_data = Vec::new();
        File::open(path)?.read_to_end(&mut image_data)?;
        let format = match image::guess_format(&image_data[..])? {
            ImageFormat::PNG => Ok(Format::Png),
            ImageFormat::JPEG => Ok(Format::Jpeg),
            _ => Err(CoverError::UnsupportedFormat),
        }?;
        Ok(Cover { image_data, format })
    }

    pub fn vw_safe_cover(&self) -> Result<Cover, CoverError> {
        let path = self.cover_path().ok_or(CoverError::NoCover)?;
        let img = image::open(path)?
            .resize(300, 300, FilterType::Lanczos3)
            .to_rgb();
        let mut image_data = Vec::new();
        JPEGEncoder::new(&mut image_data).encode(
            &img,
            img.width(),
            img.height(),
            <Rgb<u8> as Pixel>::color_type(),
        )?;
        Ok(Cover {
            image_data,
            format: Format::Jpeg,
        })
    }
}

pub enum Format {
    Png,
    Jpeg,
}

pub struct Cover {
    image_data: Vec<u8>,
    format: Format,
}

pub enum CoverError {
    NoCover,
    Io(io::Error),
    Image(ImageError),
    UnsupportedFormat,
}

impl From<io::Error> for CoverError {
    fn from(err: io::Error) -> CoverError {
        CoverError::Io(err)
    }
}

impl From<ImageError> for CoverError {
    fn from(err: ImageError) -> CoverError {
        CoverError::Image(err)
    }
}
