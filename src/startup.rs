use std::{fs, path::{PathBuf}};

use crate::db::open_connection;

pub fn sqlite_init(data_path: &PathBuf) {
    //open the file
   _= fs::File::create_new(data_path);

   //initiate all the tables
   let connection = open_connection();
let queries = [
    "CREATE TABLE IF NOT EXISTS apiKeys (key TEXT PRIMARY KEY, for TEXT);",
    "CREATE TABLE IF NOT EXISTS lists (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, leadsFetched INTEGER, emailsFetched INTEGER, filter INTEGER, next TEXT);",
    "CREATE TABLE IF NOT EXISTS filters (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, person TEXT, location TEXT, industry TEXT, keywords TEXT, employeeSize TEXT);",
    "CREATE TABLE IF NOT EXISTS leads (id TEXT, first_name TEXT, last_name TEXT, name TEXT, title TEXT, linkedin_url TEXT, state TEXT, city TEXT, org_website TEXT, country TEXT, org_name TEXT, org_fb_url TEXT, org_linkedin_url TEXT, email TEXT, listId INTEGER);",
];


for query in queries.iter() {
    connection.execute(query, []).unwrap();
}
}


