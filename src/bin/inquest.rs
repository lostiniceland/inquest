#[macro_use]
extern crate clap;

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use secrecy::SecretString;

use libinquest::crypto::encrypt_secret;
use libinquest::error::InquestError;
use libinquest::{run_from_config, ProbeReport};

struct ReportDisplay<'a, T>(&'a T);
struct ErrorDisplay<'a, T>(&'a T);

fn main() {
    stderrlog::new().verbosity(1).quiet(false).init().unwrap();

    // https://nick.groenen.me/posts/rust-error-handling/#1-simplified-result-type
    if let Err(err) = run() {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), anyhow::Error> {
    let matches = cli().get_matches();

    let key = matches.get_one::<String>("key");

    let mut x = std::env::current_dir()?;
    let config = matches
        .get_one::<String>("config")
        .map(std::path::Path::new)
        .unwrap_or({
            x.push("default.conf");
            x.as_path()
        });

    match matches.subcommand() {
        Some(("encrypt", sub_matches)) => command_encrypt(
            sub_matches
                .get_one::<String>("password")
                .unwrap()
                .to_string(),
            key.map(|k| SecretString::new(k.to_string())),
        )
        .context("Unable to encrypt secret"),
        _ => command_execute(config).context(format!(
            "Unable to run with configuration '{}'",
            config.display()
        )),
    }
}

fn cli() -> Command {
    command!()
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::new("key")
                .help("Use a custom crypto-key. Must be between 10-32 characters long.")
                .long("key")
                .short('k')
                .global(true)
                .num_args(1),
        )
        .arg(Arg::new("config").help("Location of the configuration to use."))
        .subcommand(
            Command::new("encrypt")
                .about("Encrypt a string to be used in specification")
                .arg(Arg::new("password")),
        )
}

fn command_encrypt(s: String, key: Option<SecretString>) -> Result<()> {
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
                .map(ReportDisplay)
                .collect::<Vec<ReportDisplay<ProbeReport>>>();
            let failures = failures
                .iter()
                .map(ErrorDisplay)
                .collect::<Vec<ErrorDisplay<InquestError>>>();
            if let Some(mut terminal) = term::stdout() {
                for failure in failures {
                    let color = match failure.0 {
                        InquestError::FailedExecutionError { .. } => term::color::RED,
                        InquestError::FailedAssertionError { .. } => term::color::RED,
                        InquestError::AssertionMatchingError(..) => term::color::YELLOW,
                        _ => term::color::WHITE,
                    };
                    terminal.fg(color).unwrap();
                    println!("{:#}", failure);
                }
                terminal.fg(term::color::GREEN).unwrap();
                for report in reports {
                    println!("{:#}", report);
                }
                terminal.reset()?;
            } else {
                for failure in failures {
                    println!("{:#}", failure);
                }
                for report in reports {
                    println!("{:#}", report);
                }
            }

            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

impl<'a> Display for ReportDisplay<'a, ProbeReport> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Success '{}'", self.0.probe_identifier)?;
        writeln!(f, "Acquired Data")?;
        if !self.0.data.is_empty() {
            for data in &self.0.data {
                writeln!(f, "\t{}: \n{}", data.0, data.1)?;
            }
        }
        Ok(())
    }
}

impl<'a> Display for ErrorDisplay<'a, InquestError> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            &libinquest::error::InquestError::FailedExecutionError {
                probe_identifier,
                source,
            } => {
                writeln!(f, "Failed in '{}'", probe_identifier)?;
                writeln!(f, "\tCause: {}", source)?;
            }
            &libinquest::error::InquestError::FailedAssertionError {
                probe_identifier,
                desc,
                source,
            } => {
                writeln!(f, "Failed in '{}': {}", probe_identifier, desc)?;
                writeln!(f, "\tCause: {}", source)?;
            }
            &libinquest::error::InquestError::AssertionMatchingError(desc, report) => {
                let rd = ReportDisplay(report);
                writeln!(
                    f,
                    "Assertion failed in '{}': {}",
                    rd.0.probe_identifier, desc
                )?;
            }
            e => {
                writeln!(f, "Unhandled Error: {:?}", e)?;
            }
        }
        Ok(())
    }
}
