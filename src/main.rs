extern crate maestro;

use indicatif::{ProgressBar, ProgressStyle};
use maestro::album::Album;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "maestro")]
/// Music album organization and tagging software.
struct Opt {
    #[structopt(subcommand)]
    command: Command,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    /// The path to the album. The current directory is used if not specified.
    folder: PathBuf,

    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbosity of output.
    verbose: usize,

    /// Prints out actions instead of doing them.
    #[structopt(long)]
    dry_run: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    /// Update an album's tags.
    Update,

    ///Export an album to a folder.
    Export {
        #[structopt(long, parse(from_os_str))]
        /// The root path.
        root: Option<PathBuf>,

        #[structopt(short, long, default_value = "full")]
        /// The format to export to.
        format: ExportFormat,

        #[structopt(parse(from_os_str), required_unless("root"))]
        /// The path to write the output to.
        output: Option<PathBuf>,
    },

    /// Validate an album's tags.
    Validate,

    /// Show the contents of an album.
    Show,

    /// Clear tags from the album.
    Clear,

    /// Rename files to match manifest content.
    Rename,

    /// Generate an album definition from a folder of MP3 files.
    Generate,
}

#[derive(StructOpt, Debug)]
enum ExportFormat {
    /// Export the full album (keeping ID3 tags and disc folders.)
    Full,

    /// Export the album for car use (ASCII tags and flat structure.)
    Vw,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full" => Ok(Self::Full),
            "vw" => Ok(Self::Vw),
            s => Err(format!("Invalid export format \"{}\"", s)),
        }
    }
}

fn main() {
    let Opt {
        folder,
        command,
        verbose: _verbose,
        dry_run,
    } = Opt::from_args();

    match command {
        Command::Update => {
            // TODO: Don't unwrap.
            let album = Album::load(folder).unwrap();
            let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
            let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
            let mut errors = Vec::new();

            for track in album.tracks() {
                progress_bar.set_message(format!("Updating \"{}\"...", track.title().value()));
                if let Err(e) = track.update_id3() {
                    errors.push((track, e));
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_with_message("Finished.");

            if !errors.is_empty() {
                println!("Errors:");
                for (track, error) in errors {
                    // TODO: Change to {} when UpdateId3Error implements Display + Error.
                    println!("\"{}\": {:?}", track.title().value(), error);
                }
            }
        }
        Command::Export {
            format,
            root,
            output,
        } => {
            // TODO: Don't unwrap.
            let album = Album::load(folder).unwrap();
            let output = output.unwrap_or_else(|| {
                // TODO: Don't unwrap.
                let mut root = root.unwrap();
                let artist = album.artist();
                let title = album.title();
                root.push(artist.file_safe());
                root.push(&title.file_safe());
                root
            });
            // TODO: Don't unwrap.
            std::fs::create_dir_all(&output).unwrap();

            let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
            let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
            let mut errors = Vec::new();

            for track in album.tracks() {
                progress_bar.set_message(format!("Copying \"{}\"...", track.title().value()));
                let res = match format {
                    ExportFormat::Full => track.export(&output),
                    ExportFormat::Vw => track.update_id3_vw(&output),
                };
                if let Err(e) = res {
                    errors.push((track, e));
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_with_message("Finished.");

            if !errors.is_empty() {
                println!("Errors:");
                for (track, error) in errors {
                    // TODO: Update to {} when UpdateId3VwError implements Display + Error.
                    println!("\"{}\": {:?}", track.title().value(), error);
                }
            }
        }
        Command::Validate => {
            // TODO: Don't unwrap.
            let album = Album::load(folder).unwrap();
            let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
            let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
            let mut errors = Vec::new();

            for track in album.tracks() {
                progress_bar.set_message(format!("Validating \"{}\"...", track.title().value()));
                if let Err(e) = track.validate() {
                    errors.push((track, e));
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_with_message("Finished.");

            if !errors.is_empty() {
                println!("Errors:");
                for (track, error) in errors {
                    // TODO: Print the errors better.
                    println!("\"{}\": {:?}", track.title().value(), error);
                }
            }
        }
        Command::Show => {
            let album = Album::load(folder).unwrap();
            let stdout = std::io::stdout();
            serde_yaml::to_writer(stdout, album.raw()).unwrap();
            // println!("{:#?}", album);
        }
        Command::Clear => {
            let album = Album::load(folder).unwrap();
            let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
            let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
            let mut errors = Vec::new();

            for track in album.tracks() {
                progress_bar.set_message(format!("Clearing \"{}\"...", track.title().value()));
                if let Err(e) = track.clear() {
                    errors.push((track, e));
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_with_message("Finished.");

            if !errors.is_empty() {
                println!("Errors:");
                for (track, error) in errors {
                    // TODO: Print the errors better.
                    // TODO: Change to {} when UpdateId3Error implements Display + Error.
                    println!("\"{}\": {:?}", track.title().value(), error);
                }
            }
        }
        Command::Rename => {
            use std::fs;

            let album = Album::load(folder).unwrap();

            let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
            let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
            let mut errors = Vec::new();

            for track in album.tracks() {
                progress_bar.set_message(format!("Renaming \"{}\"...", track.title().value()));
                let path = track.path();
                let can_path = track.canonical_path();
                if path != can_path && !dry_run {
                    if let Err(e) = fs::rename(path, can_path) {
                        errors.push((track, e));
                    }
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_with_message("Finished.");

            if !errors.is_empty() {
                println!("Errors:");
                for (track, error) in errors {
                    println!("\"{}\": {}", track.title().value(), error);
                }
            }
        }
        Command::Generate => {
            use std::fs;

            let album = Album::generate(folder);
            fs::create_dir_all(album.extras_path()).unwrap();
            let file = fs::File::create(album.extras_path().join("album.yaml")).unwrap();
            serde_yaml::to_writer(file, album.raw()).unwrap();
        }
    }
}
