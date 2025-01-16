use gtk4::{gdk::Display, IconTheme};
use std::env;
use super::Loader;
use super::util::SherlockError;

impl Loader {
    pub fn load_icon_theme(icon_paths: &Vec<String>)-> Result<(), SherlockError>{
        let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());
        let home_dir = env::var("HOME")
                .map_err(|e| SherlockError {
                    name:format!("Env Var Not Found Error"),
                    message: format!("Cannot unpack home directory for user."),
                    traceback: e.to_string(),
                })?;
        icon_paths
            .iter()
            .map(|path| {
                path.replace("~", &home_dir)
            })
            .for_each(|path| icon_theme.add_search_path(path));

        Ok(())
    }
}


