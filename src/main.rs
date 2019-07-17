extern crate songmaster_rs;

fn main() {}

/*

use id3::Tag;
use songmaster_rs::models::album::Album;
use std::{
    fs::{read_to_string, File, OpenOptions},
    path::PathBuf,
};
use yaml_rust::YamlLoader;

fn main() {
    let album_root = PathBuf::from("C:\\Users\\kstro\\Desktop\\CONTEMPORARY SAPPORO_new");
    let album_content = read_to_string(album_root.join("extras\\album.yaml")).unwrap();
    let album_yaml = YamlLoader::load_from_str(&album_content)
        .unwrap()
        .pop()
        .unwrap();
    let album = Album::from_yaml_and_path(album_yaml, album_root).unwrap();
    for disc in album.discs() {
        for track in disc.tracks() {
            /*
            println!("Clearing {}", track.filename());
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(track.path())
                .unwrap();
            Tag::remove_from(&mut file).unwrap();
            */
            println!("Updating {}", track.filename());
            match track.update_id3() {
                Ok(()) => println!("Update ok"),
                Err(e) => println!("Error: {:?}", e),
            }
        }
    }
}

*/
