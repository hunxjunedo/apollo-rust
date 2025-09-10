use std::{cmp::min, path::PathBuf};
use anyhow::anyhow;
use colorize::AnsiColor;
use directories::BaseDirs;
use inquire::{Select};
use prettytable::Table;
use rusqlite::{Connection, Row};

use crate::{apikey::{ApiKey, ApiKeyFor}, clear_and_logo, list::List};

pub fn open_connection() -> Connection {
    Connection::open(sqlite_path()).unwrap()
}

pub fn sqlite_path() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .data_local_dir()
        .join("apollo/apollo.sqlite")
}

pub trait DBentity {
    fn new(args: &Row) -> Self;
    fn table_name() -> &'static str;
    fn insert_new(&self) -> Result<usize, rusqlite::Error>;
    fn custom_query() -> Option<&'static str>;
}

pub trait Tablular {
    fn headers() -> Vec<&'static str>;
    fn row(&self) -> Vec<String>;
}

pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub rows: u32,
    pub total_rows: u32,
    pub rows_left: u32,
    pub next: Option<PageConfig>,
}

pub struct PageConfig {
    pub rows: u32,
    pub offset: u32,
}

fn wrap_query(user_query: String, limit: u32, offset: u32) -> String {
    // Wrap in a subquery
    format!("{} LIMIT {} OFFSET {};", user_query, limit, offset)
}

fn get_total_and_update<T>(
    config: &mut PagedResult<T>,
    connection: &Connection,
    query: String,
) -> () {
    //prepare the query
    let finalized_query = format!("SELECT COUNT(*) AS total_count FROM ({})", query);
    let rows: u32 = connection
        .query_row(&finalized_query, [], |row| row.get(0))
        .unwrap();
    config.total_rows = rows;
}

/// list all records of an entity
/// also supports pagination
pub fn list_all<T>(
    custom_query: Option<String>,
    page_config: Option<PageConfig>,
) -> Result<PagedResult<T>, rusqlite::Error>
where
    T: DBentity,
{
    let connection = open_connection();
    let query_without_pagination = match custom_query {
        Some(query) => query.to_string(),
        _ => format!("SELECT * FROM {};", T::table_name()),
    };
    let mut resultant_page = PagedResult::<T> {
        items: Vec::new(),
        rows: 0,
        total_rows: 0,
        rows_left: 0,
        next: None,
    };

    //cleanup of query
    let cleaned = query_without_pagination.trim().trim_end_matches(';');
    //get total records and mutate
    get_total_and_update(&mut resultant_page, &connection, cleaned.to_string());

    let query = match page_config {
        Some(PageConfig { offset, rows }) => {
            //get the query with limits
            wrap_query(cleaned.to_string(), rows, offset)
        }
        None => query_without_pagination,
    };

    let mut rows_result = connection.prepare(query.as_str())?;

    let rows_iterator = rows_result.query_map([], |row| Ok(T::new(row)))?;
    let mut data = Vec::new();
    let mut rows = 0;
    for row in rows_iterator {
        rows += 1;
        data.push(row.unwrap())
    }
    //no pagination

    resultant_page.items = data;
    if matches!(page_config, None) {
        resultant_page.rows = resultant_page.total_rows; //since all rows available are returned
        return Ok(resultant_page);
    };


    let page_config_sure = page_config.unwrap();
    resultant_page.rows = rows;
    resultant_page.rows_left = resultant_page
        .total_rows
        .saturating_sub(resultant_page.rows + page_config_sure.offset);
    if resultant_page.rows_left > 0 {
        let rows_in_next_page = min(page_config_sure.rows, resultant_page.rows_left);
        resultant_page.next = Some(PageConfig {
            rows: rows_in_next_page,
            offset: resultant_page.total_rows - resultant_page.rows_left,
        })
    }
    Ok(resultant_page)
}

pub fn tabular_output<T>(data: &Vec<T>, heading: String)
where
    T: Tablular,
{
    let mut table = Table::new();
    table.add_row(prettytable::Row::from(T::headers()));
    for instance in data {
        table.add_row(prettytable::Row::from(instance.row()));
    }
    clear_and_logo(heading);
    table.printstd();
}


pub fn api_keys_available(purpose: ApiKeyFor) -> Option<Vec<ApiKey>> {
        let api_keys_query = format!(
        "SELECT * FROM {} WHERE for = '{}' OR for = 'both';",
        ApiKey::table_name(),
        purpose.to_str()
    );
    let apikeys = list_all::<ApiKey>(Some(api_keys_query), None)
        .unwrap()
        .items;
    if apikeys.len() == 0 {
        println!(
            "{}",
            "you don't have any API keys, create one to continue".red()
        );
        return None;
    };
    return Some(apikeys);
}

pub enum ListSelectedResult {
    Back,
    NoLists,
    ListSelected(List)
}

pub fn list_selection() -> anyhow::Result<ListSelectedResult> {
    let all_lists = list_all::<List>(List::custom_query().map(|v| v.to_string()), None)?
        .items;
    if all_lists.len() == 0 {
        return Ok(ListSelectedResult::NoLists);
    };


    tabular_output(&all_lists, "All Lists".to_string());
    let mut all_options : Vec<String> = all_lists.iter().map(|list| list.name.clone()).collect();
    all_options.push("Back".to_string());
    let selected_list_name = Select::new("please select a list ", all_options).prompt()?;
    if selected_list_name == "Go Back" {
        return  Ok(ListSelectedResult::Back);
    }
    let selected_list = all_lists.iter().find(|list|{list.name == selected_list_name}).cloned().ok_or_else(||anyhow!("list not found error"))?;
    return  Ok(ListSelectedResult::ListSelected(selected_list));
} 