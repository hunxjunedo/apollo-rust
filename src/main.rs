use std::{
    error::Error, fs
};
mod apikey;
mod db;
mod leads;
mod list;
mod settings;
mod fetch;
mod emails;
mod viewleads;
mod startup;
use colorize::AnsiColor;
use directories::BaseDirs;
use inquire::{validator::Validation, Select};
use settings::main_settings;
use startup::{sqlite_init};

use crate::{apikey::api_key_handler, emails::fetch_emails, fetch::main_fetch, leads::fetch_leads, list::list_handler, viewleads::view_leads};

fn clear_and_logo(heading: String) {
    print!("{}[2J", 27 as char);
    println!(
        "{}\n",
        r"                           dP dP          
                           88 88          
.d8888b. 88d888b. .d8888b. 88 88 .d8888b. 
88'  `88 88'  `88 88'  `88 88 88 88'  `88 
88.  .88 88.  .88 88.  .88 88 88 88.  .88 
`88888P8 88Y888P' `88888P' dP dP `88888P' 
         88                               
         dP"
        .green()
    );
    println!("{}\n", heading.blink().bold().blue())
}

fn main() {
    //check for config dir
    let mut first_time = false;
    let datadir = BaseDirs::new().unwrap();
    let mut state = MenuState::Main;
    let mut datadirlocal = datadir.data_local_dir().join("apollo");

    if !fs::exists(&datadirlocal).unwrap() {
        first_time = true;
        _ = fs::create_dir(&datadirlocal);
    }

    //make the sqlite if not already there
    datadirlocal = datadirlocal.join("apollo.sqlite");
    if !fs::exists(&datadirlocal).unwrap() {
        sqlite_init(&datadirlocal);
    }


    loop {
        state = match state {
            MenuState::Main => main_menu(first_time),
            MenuState::Fetch => main_fetch(),
            MenuState::Settings => main_settings(),
            MenuState::APIkeys => api_key_handler(),
            MenuState::FetchEmails => fetch_emails(),
            MenuState::Lists => list_handler(),
            MenuState::FetchLeads => fetch_leads(),
            MenuState::ViewLeads => view_leads(),
            MenuState::GoodBye => break,
            _ => break,
        }
    }

    println!("{}", r"                                 dP dP                                                       dP dP          
                                 88 88                                                       88 88          
.d8888b. .d8888b. .d8888b. .d888b88 88d888b. dP    dP .d8888b.    .d8888b. 88d888b. .d8888b. 88 88 .d8888b. 
88'  `88 88'  `88 88'  `88 88'  `88 88'  `88 88    88 88ooood8    88'  `88 88'  `88 88'  `88 88 88 88'  `88 
88.  .88 88.  .88 88.  .88 88.  .88 88.  .88 88.  .88 88.  ...    88.  .88 88.  .88 88.  .88 88 88 88.  .88 
`8888P88 `88888P' `88888P' `88888P8 88Y8888' `8888P88 `88888P'    `88888P8 88Y888P' `88888P' dP dP `88888P' 
     .88                                          .88                      88                               
 d8888P                                       d8888P                       dP".green())
}

enum MenuState {
    Main,
    Settings,
    APIkeys,
    GoodBye,
    Lists,
    Fetch,
    FetchLeads,
    FetchEmails,
    ViewLeads
}

    pub fn empty_validator(v: &str) ->Result<Validation, Box<dyn Error + Send + Sync>> {
        if v.trim() == "" {
            Ok(Validation::Invalid("required field".red().into()))
        } else {
            Ok(Validation::Valid)
        }
    }

fn main_menu(first_time: bool) -> MenuState {
    clear_and_logo("Main Menu".to_string());
    if first_time {
        println!("{}","This is your first time using Apollo, please consider setting an API key first.".blue())
    }
    let options = vec!["Fetch", "Settings", "Export", "View Leads", "Exit"];
    let selection = Select::new("Settings".green().as_str(), options.clone())
        .prompt()
        .unwrap();
    match options.iter().position(|&x| x == selection) {
        Some(0) => MenuState::Fetch,
        Some(1) => MenuState::Settings,
        Some(2) => MenuState::Main,
        Some(3) => MenuState::ViewLeads,
        _ => MenuState::GoodBye,
    }
}
