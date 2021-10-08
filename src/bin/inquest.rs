#[macro_use]
extern crate clap;

use std::fmt::{Display, Formatter};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{App, Arg, SubCommand};
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
        command_encrypt(
            arg.value_of("password").unwrap().to_string(),
            key.map(|k| SecretString::new(k.to_string())),
        )
        .context("Unable to encrypt secret")
    } else {
        command_execute(config).context(format!(
            "Unable to run with configuration '{}'",
            config.display()
        ))
    }
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
                .map(|r| ReportDisplay(r))
                .collect::<Vec<ReportDisplay<ProbeReport>>>();
            let failures = failures
                .iter()
                .map(|e| ErrorDisplay(e))
                .collect::<Vec<ErrorDisplay<InquestError>>>();
            if let Some(mut terminal) = term::stdout() {
                for failure in failures {
                    let color = match failure.0 {
                        InquestError::FailedExecutionError { .. } => term::color::RED,
                        InquestError::AssertionError(_) => term::color::YELLOW,
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
        Ok(())
    }
}

impl<'a> Display for ErrorDisplay<'a, InquestError> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            &libinquest::error::InquestError::FailedExecutionError { source } => {
                writeln!(f, "Probe-Execution failed due to: {}", source)?;
            }
            &libinquest::error::InquestError::AssertionError(report) => {
                let rd = ReportDisplay(report);
                writeln!(f, "Probe-Assertion failed due to: {}", rd)?;
            }
            _ => {}
        }
        Ok(())
    }
}
