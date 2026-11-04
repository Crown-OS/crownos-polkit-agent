use std::io::{self, Read};

use anyhow::{Context, Result};
use crownos_polkit_agent::{dialog, request::DialogRequest};

fn main() -> Result<()> {
    let mut encoded = String::new();
    io::stdin()
        .read_to_string(&mut encoded)
        .context("reading the request from the agent")?;
    dialog::run(DialogRequest::decode(&encoded)?)
}
