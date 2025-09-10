use colorize::AnsiColor;
use inquire::Select;

use crate::{MenuState};

pub fn main_settings() -> MenuState {
    let options = vec!["API Keys".blue(), "Lists".blue(), "Back".red()];
    let selection = Select::new("Settings".green().as_str(), options.clone()).prompt().unwrap();
        match options.iter().position(|x| *x == selection) {
            Some(0) => MenuState::APIkeys,
            Some(1) => MenuState::Lists,
            Some(2) => MenuState::Main,
            _ => panic!("something went wrong")
        }
    }



