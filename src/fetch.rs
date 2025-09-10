use colorize::AnsiColor;
use inquire::Select;

use crate::{MenuState};

pub fn main_fetch() -> MenuState {
    let options = vec!["Leads".blue(), "Emails".blue(), "Back".red()];
    let selection = Select::new("Fetch".green().as_str(), options.clone()).prompt().unwrap();
        match options.iter().position( |x| *x == selection) {
            Some(0) => MenuState::FetchLeads,
            Some(1) => MenuState::FetchEmails,
            Some(2) => MenuState::Main,
            _ => panic!("something went wrong")
        }
    }



