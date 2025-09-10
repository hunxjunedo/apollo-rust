
use anyhow::anyhow;
use colorize::AnsiColor;
use inquire::{validator::Validation, Select, Text};
use rusqlite::{Row};

use crate::{db::{list_all, open_connection, tabular_output, DBentity, Tablular}, empty_validator, MenuState};

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key: String,
    pub purpose: ApiKeyFor
}
#[derive(Debug, Clone)]
pub enum ApiKeyFor {
    Email,
    Leads,
    Both
}

impl ApiKeyFor {
   pub fn to_str(&self) -> String {
        match self {
            Self::Email => "email",
            Self::Leads => "leads",
            _ => "both"
        }.to_string()
    }

    fn from_str(text: String) -> Self {
        match text.as_str() {
            "email" => Self::Email,
            "leads" => Self::Leads,
            _ => Self::Both
        }
    }
}

impl Tablular for ApiKey {
    fn headers() -> Vec<&'static str> {
        vec!["key", "purpose"]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.key.to_owned(),
            self.purpose.to_str()
        ]
    }
}



impl DBentity for ApiKey {
    fn new(row: &Row) -> Self {
        ApiKey { key: row.get(0).unwrap(), purpose: ApiKeyFor::from_str(row.get(1).unwrap()) }
    }
 
    fn table_name() -> &'static str {
        "apiKeys"
    }

    fn custom_query() -> Option<&'static str> {
        None
    }

    fn insert_new(&self) -> Result<usize, rusqlite::Error>  {
        open_connection().execute(
        format!("INSERT INTO {} (key, for) VALUES (?1, ?2)", Self::table_name()).as_str(),
        (self.key.to_owned(), self.purpose.to_str()))
        
    }
}

pub fn api_key_handler() -> MenuState{
    //show all api keys
    //first get all
    let all_api_keys = list_all::<ApiKey>(None, None).unwrap().items;
    let options = vec!["List API keys".blue(), "Add new".blue(), "Back".red()];
     let direction = Select::new("API Keys".green().as_str(),options.clone()).prompt().unwrap(); 
        match options.iter().position(|x|{*x == direction}) {
            Some(0) => list_all_api_keys(&all_api_keys),
            Some(1) => add_new_api_key(all_api_keys),
            _ => MenuState::Settings
        
    }
}

fn list_all_api_keys(all_api_keys: &Vec<ApiKey> )-> MenuState{
    tabular_output::<ApiKey>( all_api_keys, "API Keys".to_string());
    MenuState::APIkeys
}

fn add_new_api_key(all_api_keys: Vec<ApiKey>)-> MenuState{
    //prompt for api key
    println!("{}", "Add new API Key".green());

    let dup_validator = move |value: &str|{
        let keys = all_api_keys.iter().map(|apikey| apikey.key.to_owned()).collect::<Vec<String>>();
        if keys.contains(&value.to_string()) {
             Ok(Validation::Invalid("API key duplicated".into()))
        }else{
            Ok(Validation::Valid)
        }
        
    };

    //api key
    let api_key = Text::new("enter the API key".blue().as_str()).with_validators(&[Box::from(dup_validator), Box::from(empty_validator)]).prompt().unwrap();

    //now get the purpose
    let options = vec!["email", "leads", "both"];
    let purpose = Select::new("what is the purpose of this API key ?".blue().as_str(), options).prompt().unwrap();
    
    //create the instance
    let api_key_instance = ApiKey {
        key: api_key.trim().to_string(),
        purpose: ApiKeyFor::from_str(purpose.to_string())
    };
    
    let result = api_key_instance.insert_new();
    match result {
        Ok(_) => println!("{} \n", "added successfully".blue()),
        Err(e) => println!("{} error: {:?} \n", "couldn't add the new API key".red(), e),
    }
    MenuState::APIkeys
}

pub struct ApiKeyRotation {
   pub active_index: usize,
   pub api_keys_avaialble: Vec<ApiKey>
}

impl ApiKeyRotation {
    pub fn rotate(&mut self) -> anyhow::Result<ApiKey>{
        if self.active_index >= self.api_keys_avaialble.len() - 1 {
            Err(anyhow!("all api keys have reached thier limit"))
        }else{
            self.active_index+=1;
            Ok(self.api_keys_avaialble[self.active_index].clone())
        }
    }

    pub fn get(&self) -> ApiKey {
        self.api_keys_avaialble[self.active_index].clone()
    }
}