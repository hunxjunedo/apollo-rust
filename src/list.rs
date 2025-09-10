

use std::error::Error;

use crate::{
    MenuState,
    db::{DBentity, Tablular, list_all, open_connection, tabular_output},
    empty_validator,
};
use anyhow::{anyhow};
use colorize::AnsiColor;
use inquire::{list_option::ListOption, validator::{ Validation}, MultiSelect, Select, Text};

#[derive(Clone)]
pub enum EmployeeSize {
    Unspecified,
    Specified(Vec<SizeOptions>),
}

pub fn empty_size_validator(v: &[ListOption<&&str>]) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    if v.len() == 0 {
        Ok(Validation::Invalid("please select an option".into()))
    }else{
        Ok(Validation::Valid)
    }
}

#[derive(Clone)]
enum SizeOptions {
    Under10,
    Under20,
    Under50,
    Under100,
    Under200,
    Under500,
    Under1000,
    Under2k,
    Under5k,
    Under10k,
    Above10k,
}

impl SizeOptions {
    fn to_str(&self, for_api: bool) -> String {
        let text = match self {
            Self::Under10 => "1-10",
            Self::Under20 => "11-20",
            Self::Under50 => "21-50",
            Self::Under100 => "51-100",
            Self::Under200 => "101-200",
            Self::Under500 => "201-500",
            Self::Under1000 => "501-1000",
            Self::Under2k => "1001-2000",
            Self::Under5k => "2001-5000",
            Self::Under10k => "5001-10000",
            Self::Above10k => "10001+",
        }
        .to_string();
        if for_api {
            return text.replace("-", ",");
        }
        text
    }

    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "1-10" | "1,10" => Some(Self::Under10),
            "11-20" | "11,20" => Some(Self::Under20),
            "21-50" | "21,50" => Some(Self::Under50),
            "51-100" | "51,100" => Some(Self::Under100),
            "101-200" | "101,200" => Some(Self::Under200),
            "201-500" | "201,500" => Some(Self::Under500),
            "501-1000" | "501,1000" => Some(Self::Under1000),
            "1001-2000" | "1001,2000" => Some(Self::Under2k),
            "2001-5000" | "2001,5000" => Some(Self::Under5k),
            "5001-10000" | "5001,10000" => Some(Self::Under10k),
            "10001+" => Some(Self::Above10k),
            _ => None,
        }
    }
}

impl EmployeeSize {
   pub fn to_str(&self, for_api: bool) -> String {
        let sep = if for_api { ";" } else { ", " };
        match self {
            Self::Unspecified => "unspecified".to_string(),
            Self::Specified(val) => val
                .iter()
                .map(|a| a.to_str(for_api))
                .collect::<Vec<_>>()
                .join(sep),
        }
    }

   pub fn from_str(text: String, from_api: bool) -> EmployeeSize {
        let sep = if from_api { ";" } else { ", " };
        match text.as_str() {
            "unspecified" => EmployeeSize::Unspecified,
            text => EmployeeSize::Specified(
                text.split(sep)
                    .into_iter()
                    .map(|single_filter: &str| SizeOptions::from_str(single_filter).unwrap())
                    .collect::<Vec<SizeOptions>>(),
            ),
        }
    }
}
#[derive(Clone)]
pub struct ListFilter {
    id: u32,
    name: String,
   pub person_title: String,
   pub location: String,
   pub  industry: String,
    pub keywords: Option<String>,
   pub employee_size: EmployeeSize,
}

#[derive(Clone)]
pub struct List {
    pub id: u32,
   pub name: String,
   pub leads_fetched: u32,
   pub emails_fetched: u32,
   pub filter: ListFilter,
    pub next_pointer: Option<String>,
}

impl Tablular for List {
    fn headers() -> Vec<&'static str> {
        vec![
            "name",
            "leads fetched",
            "emails fetched",
            "person",
            "location",
            "industry",
            "keywords",
            "employee size",
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.to_owned(),
            self.leads_fetched.to_string(),
            self.emails_fetched.to_string(),
            self.filter.person_title.to_owned(),
            self.filter.location.to_owned(),
            self.filter.industry.to_owned(),
            self.filter
                .keywords
                .to_owned()
                .unwrap_or("none".to_string()),
            self.filter.employee_size.to_str(false),
        ]
    }
}

impl DBentity for List {
    fn table_name() -> &'static str {
        "lists"
    }

    fn custom_query() -> Option<&'static str> {
        Some("SELECT * FROM lists INNER JOIN filters ON lists.filter = filters.id")
    }

    fn new(row: &rusqlite::Row) -> Self {
        Self {
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
            leads_fetched: row.get(2).unwrap(),
            emails_fetched: row.get(3).unwrap(),
            next_pointer: row.get(5).unwrap(),
            filter: ListFilter {
                id: row.get(6).unwrap(),
                name: row.get(7).unwrap(),
                person_title: row.get(8).unwrap(),
                location: row.get(9).unwrap(),
                industry: row.get(10).unwrap(),
                keywords: row.get(11).unwrap(),
                employee_size: EmployeeSize::from_str(row.get(12).unwrap(), false),
            },
        }
    }

    fn insert_new(&self) -> Result<usize, rusqlite::Error> {
        //supposed to just run the insert query
        let connection = open_connection();
        connection.execute("INSERT INTO filters (name, person, location, industry, keywords, employeeSize) VALUES(?1, ?2, ?3, ?4, ?5, ?6)", 
        (format!("{}-filter", &self.name), &self.filter.person_title, &self.filter.location, &self.filter.industry,&self.filter.keywords, &self.filter.employee_size.to_str(false))
    )?;
        //get the id
        let primaryid = connection.last_insert_rowid();
        connection.execute(
            format!(
                "INSERT INTO {} (name, leadsFetched, emailsFetched, filter) VALUES (?1, ?2, ?3, ?4)",
                Self::table_name()
            )
            .as_str(),
            (
                &self.name,
                &self.leads_fetched,
                &self.emails_fetched,
                primaryid,
            ),
        )
    }
}

pub fn list_handler() -> MenuState {

    let data = list_all::<List>(List::custom_query().map(|v|{v.to_string()}), None).unwrap().items;
    let options = vec!["List all lists".blue(), "Add new".blue(), "Back".red()];
    let direction = Select::new("Lists".green().as_str(), options.clone())
        .prompt()
        .unwrap();
    match options.iter().position(|x| *x == direction) {
        Some(0) => list_all_lists(&data),
        Some(1) => add_new_list(data),
        _ => MenuState::Settings,
    }
}

fn list_all_lists(data: &Vec<List>) -> MenuState {
    tabular_output(data, "All Lists".to_string());
    MenuState::Lists
}

fn add_new_list(data: Vec<List>) -> MenuState {
    let dup_validator_list = move |val: &str| {
        let names: Vec<String> = data
            .iter()
            .map(|list: &List| list.name.clone())
            .collect();
        if names.contains(&val.to_string()) {
            Ok(Validation::Invalid(
                "list by this name already exists".into(),
            ))
        } else {
            Ok(Validation::Valid)
        }
    };
    //start taking inputs
    //name
    let name = Text::new("enter the name of the new list:".blue().as_str())
        .with_validators(&[Box::new(dup_validator_list), Box::new(empty_validator)])
        .prompt()
        .unwrap()
        .trim()
        .to_string();
    //person
    let person = Text::new("enter the title of the person:".blue().as_str())
        .with_validator(empty_validator)
        .with_placeholder("ceo")
        .prompt()
        .unwrap()
        .trim()
        .to_string();
    let location = Text::new("enter the location:".blue().as_str())
        .with_placeholder("Texas")
        .with_validator(empty_validator)
        .prompt()
        .unwrap()
        .trim()
        .to_string();
    let industry = Text::new("enter the industry name:".blue().as_str())
        .with_placeholder("construction")
        .with_validator(empty_validator)
        .prompt()
        .unwrap()
        .trim()
        .to_string();
    let keywords_in = Text::new("enter the keywords:".blue().as_str())
        .with_placeholder("roofing")
        .prompt()
        .unwrap()
        .trim()
        .to_string();
    let keywords = match keywords_in.as_str() {
        "" => None,
        val => Some(val.to_string()),
    };
    //now get the employee option
    let options = vec![
        "unspecified",
        "1-10",
        "11-20",
        "21-50",
        "51-100",
        "101-200",
        "201-500",
        "501-1000",
        "1001-2000",
        "2001-5000",
        "5001-10000",
        "10001+",
    ];
    let selected_sizes = MultiSelect::new(
        "Select employee size (use space to select multiple, enter to confirm):"
            .blue()
            .as_str(),
        options.clone(),
    )
    .with_validator(Box::new(&empty_size_validator))
    .prompt()
    .unwrap();
    let employee_size = if selected_sizes.contains(&"unspecified") {
        EmployeeSize::Unspecified
    } else {
        EmployeeSize::from_str(selected_sizes.join(", "), false)
    };

    let newlist = List {
        name: name.clone(),
        next_pointer: None,
        id: 1,
        leads_fetched: 0,
        emails_fetched: 0,
        filter: ListFilter {
            id: 1,
            name: format!("{}-filter", name),
            person_title: person,
            location,
            industry,
            keywords,
            employee_size,
        },
    };

    match newlist.insert_new() {
        Ok(_) => println!("{} \n", "added successfully".blue()),
        Err(e) => println!("{} error: {:?} \n", "couldn't add the new list".red(), e),
    }

    MenuState::Lists
}


impl List {
    pub fn update_meta(&mut self, next_pointer: Option<String>, leads_fetched: Option<u32>, emails_fetched: Option<u32>) -> anyhow::Result<()> {
        let mut query = format!("UPDATE {} SET ", List::table_name());
        let mut changes = Vec::with_capacity(3);
        if let Some(next) = next_pointer {
            changes.push(format!("next = '{}'", next));
            self.next_pointer = Some(next);
        };
        if let Some(lead_fetched) = leads_fetched {
            changes.push(format!("leadsFetched = {}", lead_fetched));
            self.leads_fetched = lead_fetched;
        };
          if let Some(emails_fetched) = emails_fetched {
            changes.push(format!("emailsFetched = {}", emails_fetched));
            self.emails_fetched = emails_fetched
        };

        if changes.len() == 0 {
           return Err(anyhow!("no changes to make"));
        };
        query.push_str(changes.join(" , ").as_str());
        query.push_str(format!(" WHERE id = {}", self.id).as_str());
       _ = open_connection().execute(query.as_str(), [])?;
       Ok(())

    }
}