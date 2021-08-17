use std::path::Path;

use hocon::HoconLoader;

use crate::Result;
use crate::ServiceSpecification;
use crate::input::parser::parse;

mod parser;
mod console;

pub fn load_hocon_config(path: &Path) -> Result<Vec<ServiceSpecification>> {
    let root = HoconLoader::new()
        .no_url_include()
        // .no_system()
        .load_file(path)?
        .hocon()
        .unwrap();

    parse(&root)
}

// TODO read config from console

