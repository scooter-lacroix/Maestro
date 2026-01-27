use std::path::PathBuf;
use leindex_core::orchestrate::parser::parse_tracks_md;

fn main() {
    let tracks_path = PathBuf::from("maestro/tracks.md");
    match parse_tracks_md(&tracks_path) {
        Ok(tracks) => {
            println!("Found {} tracks", tracks.len());
            for track in tracks {
                println!("- {} (status: {:?})", track.id, track.status);
                println!("  link: {:?}", track.link_path);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
