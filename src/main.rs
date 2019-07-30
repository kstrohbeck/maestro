extern crate songmaster_rs;

use indicatif::ProgressBar;
use rayon::prelude::*;
use songmaster_rs::album::Album;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "songmaster")]
/// Music album organization and tagging software.
struct Opt {
    #[structopt(parse(from_os_str))]
    /// The path to the album.
    folder: PathBuf,

    #[structopt(subcommand)]
    command: Command,

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
}

fn main() {
    let Opt { folder, command, verbose, dry_run } = Opt::from_args();
    match opt.command {
        Command::Update => {
            let album = Album::load(folder).unwrap();
            let tracks = album.tracks().collect::<Vec<_>>();
            let bar = ProgressBar::new(tracks.len() as u64);
            bar.set_message("Updating tracks...");

            let errs = tracks
                .par_iter()
                .filter_map(|track| {
                    let res = track.update_id3();
                    bar.inc(1);
                    res.err().map(|err| (track, err))
                })
                .collect::<Vec<_>>();

            bar.finish();

            for (track, err) in errs {
                println!("\"{}\" - {:?}", track.filename(), err);
            }
        }
        Command::ExportVw { output } => {
            let album = Album::load(folder).unwrap();
            let tracks = album.tracks().collect::<Vec<_>>();
            let bar = ProgressBar::new(tracks.len() as u64);
            bar.set_message("Copying and updating tracks...");

            let errs = tracks
                .par_iter()
                .filter_map(|track| {
                    let res = track.update_id3_vw(&output);
                    bar.inc(1);
                    res.err().map(|err| (track, err))
                })
                .collect::<Vec<_>>();

            bar.finish();

            for (track, err) in errs {
                println!("\"{}\" - {:?}", track.filename(), err);
            }
        }
        Command::Validate => {
            let album = Album::load(folder).unwrap();
            let tracks = album.tracks().collect::<Vec<_>>();
            let bar = ProgressBar::new(tracks.len() as u64);
            bar.set_message("Validating tracks...");

            let errs = tracks
                .par_iter()
                .filter_map(|track| {
                    let res = track.validate();
                    bar.inc(1);
                    res.err().map(|errs| (track, errs))
                })
                .collect::<Vec<_>>();

            bar.finish();

            for (track, errs) in errs {
                println!("\"{}\":", track.filename());
                for err in errs {
                    println!("* {:?}", err);
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
            let tracks = album.tracks().collect::<Vec<_>>();
            let bar = ProgressBar::new(tracks.len() as u64);
            bar.set_message("Clearing tracks...");

            let errs = tracks
                .par_iter()
                .filter_map(|track| {
                    let res = track.clear();
                    bar.inc(1);
                    res.err().map(|err| (track, err))
                })
                .collect::<Vec<_>>();

            bar.finish();

            for (track, err) in errs {
                println!("\"{}\" - {:?}", track.filename(), err);
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

            let tracks = album.tracks().collect::<Vec<_>>();
            let bar = ProgressBar::new(tracks.len() as u64);
            bar.set_message("Clearing tracks...");

            let errs = tracks
                .par_iter()
                .map(|track| (track, track.path(), track.canonical_path()))
                .filter(|(_, path, can_path)| path != can_path)
                .flat_map(|(track, path, can_path)| {
                    let res = if !dry_run {
                        fs::rename(path, can_path)
                    } else {
                        println!("{:?} -> {:?}", path, can_path);
                        Ok(())
                    };

                    bar.inc(1);

                    res.err().map(|err| (track, err))
                })
                .collect::<Vec<_>>();

            bar.finish();

            for (track, err) in errs {
                println!("\"{}\" - {:?}", track.filename(), err);
            }

            // If !errs.is_empty(), quit with an error.

            // TODO: Update the album.yaml.
        }
    }
}
