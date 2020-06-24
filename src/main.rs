extern crate songmaster;

use indicatif::ProgressBar;
use rayon::prelude::*;
use songmaster::album::{Album, AlbumLoadError};
use songmaster::track::Track;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "songmaster")]
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

    /// Export an album to a VW-compatible format.
    ExportVw {
        #[structopt(parse(from_os_str))]
        /// The path to write the output to.
        output: PathBuf,
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

fn main() {
    let Opt {
        folder,
        command,
        verbose,
        dry_run,
    } = Opt::from_args();

    fn foreach_track<'a, F, E>(
        album: &'a Album,
        msg: &str,
        func: F,
    ) -> Result<(), Vec<(Track<'a>, E)>>
    where
        F: Fn(&Track<'a>) -> Result<(), E> + Send + Sync,
        E: Send + Sync,
    {
        let tracks = album.tracks().collect::<Vec<_>>();
        let bar = ProgressBar::new(tracks.len() as u64);
        bar.set_message(msg);

        let errs = tracks
            .into_par_iter()
            .filter_map(|track| {
                let res = func(&track);
                bar.inc(1);
                res.err().map(|err| (track, err))
            })
            .collect::<Vec<_>>();

        bar.finish();

        if !errs.is_empty() {
            Err(errs)
        } else {
            Ok(())
        }
    }

    match command {
        Command::Update => {
            let album = Album::load(folder).unwrap();
            let errs = foreach_track(&album, "Updating tracks...", |track| track.update_id3());
            if let Err(errs) = errs {
                for (track, err) in errs {
                    println!("\"{}\" - {:?}", track.title().value(), err);
                }
            }
        }
        Command::ExportVw { output } => {
            let album = Album::load(folder).unwrap();
            let errs = foreach_track(&album, "Copying and updating tracks...", |track| {
                track.update_id3_vw(&output)
            });
            if let Err(errs) = errs {
                for (track, err) in errs {
                    println!("\"{}\" - {:?}", track.title().value(), err);
                }
            }
        }
        Command::Validate => {
            let album = Album::load(folder).unwrap();
            let errs = foreach_track(&album, "Validating tracks...", |track| track.validate());
            if let Err(errs) = errs {
                for (track, errs) in errs {
                    println!("\"{}\":", track.filename());
                    for err in errs {
                        println!("* {:?}", err);
                    }
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
            let errs = foreach_track(&album, "Clearing tracks...", |track| track.clear());
            if let Err(errs) = errs {
                for (track, err) in errs {
                    println!("\"{}\" - {:?}", track.filename(), err);
                }
            }
        }
        Command::Rename => {
            use std::fs;

            let album = Album::load(folder).unwrap();

            // Make sure that disc folders are created.
            // If there's only one disc, its path will be the album's path, so nothing will happen.
            for disc in album.discs() {
                // TODO: Get rid of the unwrap.
                fs::create_dir_all(disc.path()).unwrap();
            }

            // If there were any errors making these, quit with an error.
            let errs = foreach_track(&album, "Renaming tracks...", |track| {
                let path = track.path();
                let can_path = track.canonical_path();
                if path == can_path {
                    return Ok(());
                }
                if !dry_run {
                    fs::rename(path, can_path)
                } else {
                    Ok(())
                }
            });

            if let Err(errs) = errs {
                for (track, err) in errs {
                    println!("\"{}\" - {:?}", track.filename(), err);
                }
            }

            // TODO: Update the album.yaml.
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
