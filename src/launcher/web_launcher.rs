

#[derive(Clone, Debug)]
pub struct Web{
    pub alias: Option<String>,
    pub method: String ,
    pub uuid: String,
    pub name: String,
    pub icon: String,
    pub engine: String,
    pub priority: u32,
    pub r#async: bool,
}

