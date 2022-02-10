extern crate maestro;

use anyhow::{Context, Result as AnyhowResult};
use indicatif::{ProgressBar, ProgressStyle};
use maestro::{album::Album, track::Track};
use std::{fmt::Debug, path::PathBuf};
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

fn run_all_tracks<F, E>(folder: PathBuf, action: &'static str, mut func: F) -> AnyhowResult<()>
where
    F: FnMut(&Track) -> Result<(), E>,
    // TODO: Change to Error + Display.
    E: Debug,
{
    run_all_tracks_with_ctx(folder, action, |_| (), |_, track| func(track))
}

fn run_all_tracks_with_ctx<F, G, T, E>(
    folder: PathBuf,
    action: &'static str,
    mut ctx: G,
    mut func: F,
) -> AnyhowResult<()>
where
    G: FnOnce(&Album) -> T,
    F: FnMut(&mut T, &Track) -> Result<(), E>,
    E: Debug,
{
    let album = Album::load(folder).context("Couldn't load album")?;
    let mut data = ctx(&album);
    let style = ProgressStyle::default_bar().template("{bar} ({pos}/{len}): {msg}");
    let progress_bar = ProgressBar::new(album.num_tracks() as u64).with_style(style);
    let mut errors = Vec::new();

    for track in album.tracks() {
        progress_bar.set_message(format!("{} \"{}\"...", action, track.title().value()));
        if let Err(e) = func(&mut data, &track) {
            errors.push((track, e));
        }
        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("Finished.");

    if !errors.is_empty() {
        println!("Errors:");
        for (track, error) in errors {
            // TODO: Change to {} when we change the bound on E.
            println!("\"{}\": {:?}", track.title().value(), error);
        }
    }

    Ok(())
}

fn main() -> AnyhowResult<()> {
    let Opt {
        folder,
        command,
        verbose: _verbose,
        dry_run,
    } = Opt::from_args();

    match command {
        Command::Update => run_all_tracks(folder, "Updating", |track| track.update_id3()),
        Command::Export {
            format,
            root,
            output,
        } => {
            run_all_tracks_with_ctx(
                folder,
                "Copying",
                |album| {
                    output.unwrap_or_else(|| {
                        // TODO: Don't unwrap.
                        let mut root = root.unwrap();
                        let artist = album.artist();
                        let title = album.title();
                        root.push(artist.file_safe());
                        root.push(&title.file_safe());
                        root
                    })
                },
                |output, track| match format {
                    ExportFormat::Full => track.export(&output),
                    ExportFormat::Vw => track.update_id3_vw(&output),
                },
            )
        }
        Command::Validate => run_all_tracks(folder, "Validating", |track| track.validate()),
        Command::Show => {
            let album = Album::load(folder).context("Couldn't load album")?;
            let stdout = std::io::stdout();
            serde_yaml::to_writer(stdout, album.raw()).context("Couldn't serialize album to yaml")
            // println!("{:#?}", album);
        }
        Command::Clear => run_all_tracks(folder, "Clearing", |track| track.clear()),
        Command::Rename => {
            run_all_tracks(folder, "Renaming", |track| {
                // TODO: Move rename() into track.
                let path = track.path();
                let can_path = track.canonical_path();
                if path != can_path && !dry_run {
                    std::fs::rename(path, can_path)
                } else {
                    Ok(())
                }
            })
        }
        Command::Generate => {
            use std::fs;

            let album = Album::generate(folder);
            fs::create_dir_all(album.extras_path()).context("Couldn't create extras folder")?;
            let file = fs::File::create(album.extras_path().join("album.yaml"))
                .context("Couldn't create album.yaml")?;
            serde_yaml::to_writer(file, album.raw()).context("Couldn't write album to file")
        }
    }
}
