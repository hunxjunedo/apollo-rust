use crate::{
    apikey::{ApiKeyFor, ApiKeyRotation}, db::{api_keys_available, list_all, list_selection, DBentity, ListSelectedResult, PageConfig}, empty_validator, leads::{header_constructor, num_validator, Lead}, list::List, MenuState
};
use anyhow::anyhow;
use colorize::AnsiColor;
use inquire::{Text, validator::Validation};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use url::Url;

pub fn fetch_emails() -> MenuState {
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

    //how many emails to fetch
    let mut selected_list = selected_list_maybe.unwrap();
    if selected_list.leads_fetched == 0 {
        println!(
            "{}",
            "you do not have any leads to fetch emails of, first fetch some leads.".red()
        );
        return MenuState::Fetch;
    };
    if selected_list.leads_fetched == selected_list.emails_fetched {
        println!("{}", "all leads already have emails checked.".red());
        return MenuState::Main;
    };
    //ask how many emails to fetch
    let emails_count = Text::new("how many emails do you want to fetch ?")
        .with_validators(&[
            Box::new(empty_validator),
            Box::new(num_validator),
            Box::new(move |v: &str| {
                let to_fetch: u32 = v.parse::<u32>().unwrap();
                let max_possible = selected_list.leads_fetched - selected_list.emails_fetched;
                if to_fetch > (max_possible) {
                    return Ok(Validation::Invalid(
                        format!(
                            "the maximum number of emails that can be fetched is {}",
                            max_possible
                        )
                        .into(),
                    ));
                } else {
                    return Ok(Validation::Valid);
                }
            }),
        ])
        .prompt()
        .unwrap();

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let maybe_api_keys = api_keys_available(ApiKeyFor::Email);
    if matches!(maybe_api_keys, None) {
        return MenuState::APIkeys;
    };
    let apikeys = maybe_api_keys.unwrap();

    let mut api_config = ApiKeyRotation {
        active_index: 0,
        api_keys_avaialble: apikeys,
    };
    //find the leads that need thier emails fetched
    let leads = list_all::<Lead>(
        Some(format!("SELECT * FROM {} WHERE listId = {}",Lead::table_name(),&selected_list.id)),
        Some(PageConfig {
            rows: emails_count.parse().unwrap(),
            offset: selected_list.emails_fetched,
        }),
    )
    .unwrap()
    .items;

    for lead in leads {
        let website_maybe = lead.org_website;
        if matches!(website_maybe, None) {
            if let Err(e) = increment_list_emails_count(&mut selected_list) {
                println!("couldn't update list meta for emails fetched {}", e);
                break;
            }
            continue;
        }
        let website = website_maybe.unwrap();
        let filter = Regex::new(r"^https?://(www\.)?").unwrap();
        let email_result = find_email(
            lead.first_name.to_lowercase(),
            lead.last_name.to_lowercase(),
            filter.replace(&website, "").to_string(),
            &mut api_config,
            &client,
        );
        match email_result {
            Err(e) => {
                println!("something went wrong while finding email: {}", e);
                break;
                //DONT update the list meta, this one is not fetched
            }
            Ok(Some(email_confirmed)) => {
                let update_result = Lead::update_email(lead.id, email_confirmed.clone());
                println!("email confirmed {}", email_confirmed);
                if let Err(e) = update_result {
                    println!("couldn't update email for a lead {}", e);
                    break;
                }
            }
            _ => {}
        }

        if let Err(e) = increment_list_emails_count(&mut selected_list) {
            println!("couldn't update list meta for emails fetched {}", e);
            break;
        }
    }

    MenuState::GoodBye
}

fn increment_list_emails_count(list: &mut List) -> anyhow::Result<()> {
    list.emails_fetched = list.emails_fetched + 1;
    list.update_meta(None, None, Some(list.emails_fetched))
}

fn find_email(
    first_name: String,
    last_name: String,
    domain: String,
    api_config: &mut ApiKeyRotation,
    client: &Client,
) -> anyhow::Result<Option<String>> {
    let potential_addresses = Vec::from([
        format!("{}@{}", last_name, domain),
        format!("{}@{}", first_name, domain),
        format!("{}{}@{}", &first_name[..=0], last_name, domain),
    ]);

    for canditate_address in potential_addresses {
        let is_valid = single_address_validity_check(
            canditate_address.as_str(),
            api_config,
            client,
        );
        match is_valid {
            Err(e) => return Err(e),
            Ok(is_valid) if is_valid => {
                return Ok(Some(canditate_address));
            }
            _ => continue,
        }
    }

    //if we've reached here, no address was valid
    return Ok(None);
}

// keep rotating api in the loop until it works or ultimately fails
// if api key is invalid, rotate it and call yourself, if its the last, return Err

fn single_address_validity_check(
    email: &str,
    api_config: &mut ApiKeyRotation,
    client: &Client,
) -> anyhow::Result<bool> {
    //make the api request
    let base_url = "validect-email-verification-v1.p.rapidapi.com";
    let request_url =
        Url::parse(format!("https://{}/v1/verify?email={}", base_url, email).as_str()).unwrap();
    println!("{}", request_url);
    let resp = client
        .get(request_url)
        .headers(header_constructor(api_config, base_url))
        .send();
    match resp {
        Err(e) => {
            println!("{}: {}", "something went wrong".red(), e.to_string().red());
            return Err(e.into());
        }
        Ok(ref r) if r.status() == 429 => {
            println!("{}", "api key limit reached".red());
            println!("{}", "rotating api key...".blue());
            let rotation_result = api_config.rotate();
            match rotation_result {
                Err(e) => {
                    println!("{}", e);
                    return Err(e);
                }
                Ok(_) => {
                    println!("{}", "successfuly rotated API key".blue());
                    return single_address_validity_check(email, api_config, client);
                }
            }
        }
        Ok(r) if !r.status().is_success() => {
            println!(
                "{}: {}",
                "something went wrong".red(),
                r.text().unwrap_or("".to_string())
            );
            return Err(anyhow!("something went wrong"));
        }
        Ok(_) => {}
    }
    #[derive(Deserialize)]
    struct EmailValidationResponse {
        status: String,
    }
    let valid_response = resp.unwrap().json::<EmailValidationResponse>()?;

    return Ok(valid_response.status == "valid" || valid_response.status == "accept_all");
}
