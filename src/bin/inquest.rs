#[macro_use]
extern crate clap;

use std::fmt::{Display, Formatter};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{App, Arg, SubCommand};
use secrecy::SecretString;

use libinquest::crypto::encrypt_secret;
use libinquest::{run_from_config, ProbeReport};

struct ReportDisplay<'a, T>(&'a T);

fn main() {
    stderrlog::new().verbosity(1).quiet(false).init().unwrap();

    if let Err(err) = run() {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), anyhow::Error> {
    let matches = App::new("Inquest")
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("key")
                .help("Use a custom crypto-key. Must be between 10-32 characters long.")
                .long("key")
                .short("k")
                .global(true)
                .takes_value(true),
        )
        .arg(Arg::with_name("config").help("Location of the configuration to use."))
        .subcommand(
            SubCommand::with_name("encrypt")
                .help("Encrypt a string to be used in specification")
                .arg(Arg::with_name("password")),
        )
        .get_matches();

    let key = matches.value_of("key");

    let mut x = std::env::current_dir()?;
    let config = matches
        .value_of("config")
        .map(|path| std::path::Path::new(path))
        .unwrap_or({
            x.push("default.conf");
            x.as_path()
        });

    if let ("encrypt", Some(arg)) = matches.subcommand() {
        command_encrypt(arg.value_of("password").unwrap().to_string(), key)
            .context("Unable to encrypt secret")
    } else {
        command_execute(config).context(format!(
            "Unable to run with configuration '{}'",
            config.display()
        ))
    }
}

fn command_encrypt(s: String, key: Option<&str>) -> Result<()> {
    let encrypted = encrypt_secret(SecretString::new(s), key)?;
    println!("Encrypted Secret: {}", encrypted);
    Ok(())
}

fn command_execute(config: &Path) -> Result<()> {
    // File::open(config)?; // until https://github.com/mockersf/hocon.rs/issues/47 fixed
    match run_from_config(config) {
        Ok((reports, failures)) => {
            let reports = reports
                .iter()
                .map(|r| ReportDisplay(r))
                .collect::<Vec<ReportDisplay<ProbeReport>>>();
            let mut terminal = term::stdout().unwrap(); // FIXME only works with -it in Docker. Do a proper match
            terminal.fg(term::color::RED).unwrap();
            println!("{:#?}", failures);
            terminal.fg(term::color::GREEN).unwrap();
            for report in reports {
                println!("{:#}", report);
            }
            terminal.reset()?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

impl<'a> Display for ReportDisplay<'a, ProbeReport> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Probe {} for '{}'",
            self.0.probe_name, self.0.probe_identifier
        )?;
        writeln!(f, "Acquired Data")?;
        if !self.0.data.is_empty() {
            for data in &self.0.data {
                writeln!(f, "\t{}: {}", data.0, data.1)?;
            }
        }
        if !self.0.metrics.is_empty() {
            writeln!(f, "Acquired Metrics")?;
            for metric in &self.0.metrics {
                writeln!(f, "\t{}:  {}", metric.0, metric.1)?;
            }
        }
        Ok(())
    }
}
