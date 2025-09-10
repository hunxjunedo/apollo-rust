

use inquire::Confirm;
use crate::{db::{list_all, list_selection, tabular_output, DBentity, ListSelectedResult}, leads::Lead, list::List, MenuState};

pub fn view_leads() -> MenuState {
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
    let selected_list = selected_list_maybe.unwrap();

    //fetch all the leads from db
    let all_leads = list_all::<Lead>(Some(format!("SELECT * FROM {} WHERE listId = {}",Lead::table_name(),&selected_list.id)), None);
    match all_leads {
        Err(e) => println!("{}: {}", "couldn't fetch leads from DB", e),
        Ok(data) => tabular_output(&data.items, format!("all leads for the list {}", selected_list.name)),
    };

    //dont go back and hide the  table
    let _ = Confirm::new("go back ?").with_default(true).prompt();
    return MenuState::Main;

}