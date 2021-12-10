//! AdventureScript is a crate for creating text-based games.
//!
//! If you just want to run a game, the `AdventureScriptGame` struct handles all needed processes.
//! Example:
//! ```no_run
//! let mut game = AdventureScriptGame::new("path_to_game".to_string(), None);
//! game.run();
//! ```
//!
//! If what you want is to make a module, you can find the public API for the AdventureScript core in
//! the `core` module, and the macros available in the crate will help keep your code readable.

//TODO: update when modules are a thing ^

pub mod core;

pub(crate) mod formats;
mod macros;
mod parsing;
mod inventory {}

use crate::core::{error::manage_error, main_commands, AdventureIO, CmdSet, GameInfo};
use std::path::PathBuf;

/// A struct that handles initializing and running an AdventureScript game.
pub struct AdventureScriptGame {
    info: GameInfo,
    commands: CmdSet,
}

impl AdventureScriptGame {
    /// document this better later, me
    /// however, root_dir is basically the root folder of the game
    pub fn new(root_dir: String, io: Option<AdventureIO>, is_local: bool) -> AdventureScriptGame {
        AdventureScriptGame {
            info: GameInfo::create(PathBuf::from(root_dir), io.unwrap_or_default(), is_local),
            commands: CmdSet::new(),
        }
    }

    pub fn run(&mut self) {
        //load config file
        if let Err(err) = self.info.load_config() {
            manage_error(&self.info, err);
            return;
        };
        println!("AdventureScript v{}\n", env!("CARGO_PKG_VERSION"));
        //add basic commands
        self.commands.extend(main_commands());
        //load script file
        if let Err(err) = self.info.load_script(None) {
            manage_error(&self.info, err);
            return;
        };
        //parser and stuff
        while !self.info.quitting {
            match parsing::parse_line(&mut self.info, &self.commands) {
                Ok(_) => (),
                Err(err) => {
                    manage_error(&self.info, err);
                    return;
                }
            };
            self.info.next_line();
        }
    }
}
