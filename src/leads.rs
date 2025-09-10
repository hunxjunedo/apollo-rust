use std::{error::Error, time::Duration};

use anyhow::{ Result};
use colorize::AnsiColor;
use inquire::{Text, validator::Validation};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use rusqlite::Connection;
use serde::{Deserialize, Deserializer};
use url::Url;
use crate::{apikey::ApiKeyFor, db::{list_selection, ListSelectedResult}};
use crate::{
    apikey::{ApiKeyRotation}, db::{api_keys_available, open_connection, DBentity, Tablular}, empty_validator, list::{EmployeeSize, List, ListFilter}, MenuState
};

pub fn num_validator(v: &str) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    let num = v.trim().parse::<u32>();
    match num {
        Ok(_) => Ok(Validation::Valid),
        _ => Ok(Validation::Invalid(
            "please enter a positive integer".red().into(),
        )),
    }
}

//input

pub fn fetch_leads() -> MenuState {
    let selection_maybe = list_selection();
    let selected_list_maybe: Option<List>;
    if let Err(e) = selection_maybe {
        println!("something went wrong {}", e);
        return MenuState::GoodBye;
    }
    let selection = selection_maybe.unwrap();
    match selection {
        ListSelectedResult::Back => {return MenuState::Fetch},
        ListSelectedResult::NoLists => {return MenuState::Lists},
        ListSelectedResult::ListSelected(list) => {selected_list_maybe = Some(list)}
    };

    let mut selected_list = selected_list_maybe.unwrap();

    //now ask how many leads to fetch
    let fetch_count = Text::new("how many leads do you want to fetch ?")
        .with_validators(&[Box::new(empty_validator), Box::new(num_validator)])
        .prompt()
        .unwrap()
        .parse::<u32>()
        .unwrap();
    println!(
        "fetching {} leads for list {}",
        fetch_count, selected_list.name
    );

    let maybe_api_keys = api_keys_available(ApiKeyFor::Leads);
    if matches!(maybe_api_keys, None){
        return MenuState::APIkeys;
    }
    let apikeys = maybe_api_keys.unwrap();
    let mut api_key_config = ApiKeyRotation {
        active_index: 0,
        api_keys_avaialble: apikeys,
    };

    //url
    let base_url = url_parser(&selected_list);
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    leads_fetcher_from_api(
        &mut selected_list,
        fetch_count,
        client,
        0,
        open_connection(),
        &mut api_key_config,
        base_url,
    );
    MenuState::Fetch
}

//structures

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        return format!("{}...", &s[..max]);
    } else {
        s.to_string()
    }
}

fn empty_string_is_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
}

#[derive(Deserialize)]
pub struct Lead {
    pub id: String,
    #[serde(rename(deserialize = "firstName"))]
    pub first_name: String,
    #[serde(rename(deserialize = "lastName"))]
    pub last_name: String,
    name: String,
    title: String,
    #[serde(rename(deserialize = "linkedinUrl"))]
    linkedin_url: String,
    #[serde(deserialize_with = "empty_string_is_none")]
    state: Option<String>,
    #[serde(deserialize_with = "empty_string_is_none")]
    city: Option<String>,
    country: String,
    #[serde(rename(deserialize = "organizationName"))]
    org_name: String,
    #[serde(rename(deserialize = "organizationWebsiteUrl"), deserialize_with = "empty_string_is_none")]
    pub org_website: Option<String>,
    #[serde(
        rename(deserialize = "organizationFacebookUrl"),
        deserialize_with = "empty_string_is_none"
    )]
    org_fb_url: Option<String>,
    #[serde(
        rename(deserialize = "organizationLinkedinUrl"),
        deserialize_with = "empty_string_is_none"
    )]
    org_linkedin_url: Option<String>,
    #[serde(skip)]
    pub email: Option<String>,
    #[serde(skip)]
    list_id: u32,
}

impl Lead {
   pub fn update_email(lead_id: String, email: String) -> Result<()>{
        let query = format!("UPDATE {} SET email = ?1 WHERE id='{}';", Self::table_name(), lead_id);
       open_connection().execute(&query, [email])?;
       anyhow::Ok(())
       
    }
}

impl DBentity for Lead {
    fn custom_query() -> Option<&'static str> {
        None
    }

    fn table_name() -> &'static str {
        "leads"
    }

    fn new(args: &rusqlite::Row) -> Self {
        Self {
            id: args.get(0).unwrap(),
            first_name: args.get(1).unwrap(),
            last_name: args.get(2).unwrap(),
            name: args.get(3).unwrap(),
            title: args.get(4).unwrap(),
            linkedin_url: args.get(5).unwrap(),
            state: args.get(6).unwrap(),
            city: args.get(7).unwrap(),
            org_website: args.get(8).unwrap(),
            country: args.get(9).unwrap(),
            org_name: args.get(10).unwrap(),
            org_fb_url: args.get(11).unwrap(),
            org_linkedin_url: args.get(12).unwrap(),
            email: args.get(13).unwrap(),
            list_id: args.get(14).unwrap(),
        }
    }

    fn insert_new(&self) -> Result<usize, rusqlite::Error> {
        open_connection().execute(
        format!("INSERT INTO {} (id, first_name, last_name, name, title, linkedin_url, state, city, org_website, country, org_name, org_fb_url, org_linkedin_url, email, listId) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)", Self::table_name()).as_str(),
        (&self.id, &self.first_name, &self.last_name, &self.name, &self.title, &self.linkedin_url, &self.state, &self.city, &self.org_website, &self.country, &self.org_name, &self.org_fb_url, &self.org_linkedin_url, &self.email, &self.list_id))
    }
}

impl Tablular for Lead {
    fn headers() -> Vec<&'static str> {
        vec![
            "name",
            "title",
            "LinkedIn URL",
            "city",
            "state",
            "country",
            "email",
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            truncate(&self.title, 7),
            self.linkedin_url.clone(),
            truncate(
                &self.city.clone().unwrap_or("not available".to_string()),
                10,
            ),
            self.state.clone().unwrap_or("not available".to_string()),
            self.country.clone(),
            self.email.clone().unwrap_or("not available".to_string()),
        ]
    }
}

//the whole fetching logic

#[derive(Deserialize)]
struct LeadsApiResult {
    #[serde(deserialize_with = "empty_string_is_none")]
    next: Option<String>,
    total: u32,
    people: Vec<Lead>,
}

impl LeadsApiResult {
    fn insert_all(&self) {
        self.people.iter().for_each(|person| {
            let insertion = person.insert_new();
            match insertion {
                Err(e) => {
                    println!("{}: {}", "couldn't insert leads into DB ".red(), e);
                    panic!()
                }
                _ => (),
            }
        });
    }
}

fn leads_fetcher_from_api(
    list: &mut List,
    count: u32,
    client: Client,
    mut fetched_count: u32,
    connection: Connection,
    api_key_config: &mut ApiKeyRotation,
    mut url: Url,
) -> MenuState {
    //1. send api request
    //2. update pointer of the list, in the db as well
    //3. save data to db, stop if needed
    //4. if needs more, return a recursive call with fetchedCount

    if list.leads_fetched > 0 && matches!(list.next_pointer, None) {
        println!(
            "{}",
            "cannot fetch leads, no more data available to fetch".red()
        )
    }

    //next pointer
    make_url_with_next(&mut url, &list);
    println!("{}", url);
    let resp = client
        .get(url.clone())
        .headers(header_constructor(&api_key_config, "apollo-api-pro.p.rapidapi.com"))
        .send();
    match resp {
        Err(e) => {
            println!("{}: {}", "something went wrong".red(), e.to_string().red());
            return MenuState::FetchLeads;
        }
        Ok(ref r) if r.status() == 429 => {
            println!("{}", "api key limit reached".red());
            println!("{}", "rotating api key...".blue());
            let rotation_result = api_key_config.rotate();
            match rotation_result {
                Err(e) => {
                    println!("{}", e);
                   return MenuState::FetchLeads;
                }
                Ok(_) => {
                    println!("{}", "successfuly rotated API key".blue());
                    return leads_fetcher_from_api(
                        list,
                        count,
                        client,
                        fetched_count,
                        connection,
                        api_key_config,
                        url,
                    );
                }
            }
        }
        Ok( r) if !r.status().is_success() => {
            println!("{}: {}", "something went wrong".red(), r.text().unwrap_or("".to_string()));
            return MenuState::FetchLeads;
        }
        Ok(_) => {}
    }

    //parsing
    let deserialized_resp_maybe = resp.unwrap().json::<LeadsApiResult>();
    if let Err(e) = &deserialized_resp_maybe {
        println!("{}: {}", "Failed to deserialize response".red(), e);
        return MenuState::FetchLeads;
    }

    let mut deserialized_resp: LeadsApiResult = deserialized_resp_maybe.unwrap();
    deserialized_resp
        .people
        .iter_mut()
        .for_each(|person| person.list_id = list.id.to_owned());
    deserialized_resp.insert_all();

    fetched_count += deserialized_resp.total;
    //update the next pointer and leadsFetched, pass the next pointer or stop
    let update_result = list.update_meta(
        deserialized_resp.next,
        Some(&list.leads_fetched + deserialized_resp.total),
        None,
    );
    match update_result {
        Err(e) => {
            println!("{}", e);
            return MenuState::FetchLeads;
        }
        Ok(_) => println!("{}", "successfully update the meta".blue()),
    }

    if fetched_count >= count {
        println!(
            "successfuly fetched {} leads for the list {}",
            fetched_count, list.name
        );
        return MenuState::Main;
    }
    return leads_fetcher_from_api(
        list,
        count,
        client,
        fetched_count,
        connection,
        api_key_config,
        url,
    );
}

fn url_parser(list: &List) -> Url {
    let mut base_url_text = "https://www.apollo-api-pro.p.rapidapi.com".to_string();
    let ListFilter {
        person_title,
        location,
        industry,
        employee_size,
        keywords,
        ..
    } = &list.filter;
    base_url_text = format!(
        "{}/page?locations={}&industry={}&personTitle={}",
        base_url_text, location, industry, person_title
    );
    if !matches!(employee_size, EmployeeSize::Unspecified) {
        base_url_text = format!(
            "{}&numEmployees={}",
            base_url_text,
            employee_size.to_str(true)
        );
    };
    if let Some(keywords) = keywords {
        base_url_text = format!("{}&qKeywords={}", base_url_text, keywords);
    };

    let base_url = Url::parse(base_url_text.as_str()).unwrap();
    base_url
}

fn make_url_with_next(url: &mut Url, list: &List) {
    if let Some(next) = &list.next_pointer {
        let mut new_query: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(name, _)| name != "next")
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect();
        new_query.push(("next".to_string(), next.to_string()));
        url.query_pairs_mut().clear().extend_pairs(&new_query);
    }
}

pub fn header_constructor(api_key_config: &ApiKeyRotation, api_host: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-rapidapi-key",
        HeaderValue::from_str(api_key_config.get().key.as_str()).unwrap(),
    );
    headers.insert(
        "x-rapidapi-host",
        HeaderValue::from_str(api_host).unwrap() 
    );
    headers
}
