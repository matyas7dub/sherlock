use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct AppData{
    pub icon: String,
    pub exec: String,
    pub search_string: String,
}

#[derive(Clone, Debug)]
pub struct App{
    pub alias: Option<String>,
    pub method: String ,
    pub name: String,
    pub r#async: bool,
    pub priority: u32,
    pub apps: HashMap<String, AppData>,
}

