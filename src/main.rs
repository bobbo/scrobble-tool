#[macro_use]
extern crate bitflags;
extern crate discogs;
extern crate getopts;
extern crate rustfm_scrobble;

#[macro_use] extern crate log;
extern crate env_logger;

use std::env;

use discogs::Discogs;
use getopts::Options;
use rustfm_scrobble::{Scrobbler, Scrobble, ScrobbleBatch};

bitflags! {
    struct ScrobbleType: u8 {
        const ALBUM = 0x01;
        const TRACK = 0x02;
    }
}

const API_KEY:&'static str = "65eeafc3adfdb1c1dbad47332014ccbc";
const API_SECRET:&'static str = "799127ee2d8a5a7099bff73bbc7b9a8e";

trait InfoSource {
    fn init(&self) -> Result<(), String>;
    fn get_capabilities(&self) -> ScrobbleType;
    fn get_metadata(&self, opts: &Opts) -> Result<ScrobbleBatch, String>;
}

struct OptsInfoSource {}

impl InfoSource for OptsInfoSource {

    fn init(&self) -> Result<(), String> {
        Ok(())
    }

    fn get_capabilities(&self) -> ScrobbleType {
        ScrobbleType::TRACK
    }

    fn get_metadata(&self, opts: &Opts) -> Result<ScrobbleBatch, String> {
        let track = opts.track.clone().ok_or("Track name must be set")?;
        let artist = opts.artist.clone().ok_or("Artist name must be set")?;
        let album = opts.album.clone().ok_or("Album name must be set")?;

        Ok(ScrobbleBatch::from(vec!(Scrobble::new(artist, track, album))))
    }

}

struct DiscogsInfoSource {
    discogs: Option<Discogs>
}

impl InfoSource for DiscogsInfoSource {

    fn init(&self) -> Result<(), String> {
        let source = DiscogsInfoSource {
            discogs: Some(Discogs::new("ScrobbleTool/0.1"))
        };

        Ok(())
    }

    fn get_capabilities(&self) -> ScrobbleType {
        ScrobbleType::TRACK
    }

    fn get_metadata(&self, opts: &Opts) -> Result<ScrobbleBatch, String> {
        let track = opts.track.clone().ok_or("Track name must be set")?;
        let artist = opts.artist.clone().ok_or("Artist name must be set")?;
        let album = opts.album.clone().ok_or("Album name must be set")?;

        Ok(ScrobbleBatch::from(vec!(Scrobble::new(artist, track, album))))
    }

}

#[derive(Debug)]
struct Opts {
    scrobble_type: ScrobbleType,
    artist: Option<String>,
    track: Option<String>,
    album: Option<String>,

    discogs_id: Option<String>,

    username: Option<String>,
    password: Option<String>,

    dry_run: bool
}

impl Opts {

    fn new() -> Result<Opts, String> {
        let mut opt_config = Options::new();
        opt_config.optopt("u", "username", "Last.fm username", "USERNAME");
        opt_config.optopt("p", "password", "Last.fm password", "PASSWORD");
        opt_config.optopt("", "artist", "The artist name", "ARTIST");
        opt_config.optopt("", "track", "The track name", "TRACK");
        opt_config.optopt("", "album", "The album name", "ALBUM");
        opt_config.optopt("", "discogs-id", "Discogs ID of release", "DISCOGS_ID");
        opt_config.optopt("t", "type", "Sets scrobble type to track or album (defaults to single track)", "TYPE");
        opt_config.optflag("", "dry-run", "Dry run (stop before actually scrobbling)");

        let args: Vec<String> = env::args().collect();
        let matches = match opt_config.parse(&args[1..]) {
            Ok(m) => { m }
            Err(f) => { return Err(format!("{}", f)) }
        };
        
        let scrobble_type = match matches.opt_str("type") {
            Some(ref val) if val == "track" => {
                ScrobbleType::TRACK
            },
            Some(ref val) if val == "album" => {
                ScrobbleType::ALBUM
            },
            _ => {
                warn!("Failed to parse type option, defaulting to single track");
                ScrobbleType::TRACK
            }
        };
        
        Ok(Opts {
            scrobble_type: scrobble_type,
            artist: matches.opt_str("artist"),
            track: matches.opt_str("track"),
            album: matches.opt_str("album"),

            discogs_id: matches.opt_str("discogs-id"),

            username: matches.opt_str("username"),
            password: matches.opt_str("password"),

            dry_run: matches.opt_present("dry-run")
        })
    }

}

fn main() {
    env_logger::init();

    let opts: Opts;
    match Opts::new() {
        Ok(parsed_opts) => {
            debug!("Parsed opts: {:?}", parsed_opts);
            opts = parsed_opts;
        },
        Err(err) => {
            panic!("Failed to parse opts: {}", err);
        }
    }

    let info_source: &InfoSource = if let Some(_) = opts.discogs_id {
        &DiscogsInfoSource{discogs: None}
    } else {
        &OptsInfoSource{}
    };

    if !info_source.get_capabilities().intersects(opts.scrobble_type) {
        panic!("Info source does not support requested scrobble type {:?}", opts.scrobble_type);
    }

    let scrobbles = match info_source.get_metadata(&opts) {
        Ok(scrobbles) => scrobbles,
        Err(err) => {
            panic!("Failed to fetch metadata: {}", err);
        }
    };

    info!("Got scrobbles: {:?}", scrobbles);

    let mut scrobbler = Scrobbler::new(API_KEY.to_string(), API_SECRET.to_string());
    match scrobbler.authenticate_with_password(opts.username.unwrap(), opts.password.unwrap()) {
        Ok(session) => {
            debug!("Authenticated with Last.fm");
            info!("Session key: {}", session.key);
        },
        Err(err) => {
            panic!("Failed to authenticate with Last.fm: {}", err);
        }
    }

    if !opts.dry_run {
        match scrobbler.scrobble_batch(scrobbles) {
            Ok(_) => {
                println!("Done!");
            },
            Err(err) => {
                error!("Scrobbling failed: {:?}", err);
            }
        }
    }
}
