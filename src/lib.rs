#[cfg(test)]
#[macro_use]
extern crate assert_matches;

use std::path::Path;
use std::result;
use std::time::Duration;

use secrecy::SecretString;
use url::Url;

use crate::crypto::decrypt_secret;
use crate::error::InquestError;

pub mod crypto;
pub mod error;
mod input;
mod probes;

const GO: GlobalOptions = GlobalOptions {
    timeout: Duration::from_secs(30),
};

/// A 'Probe' is implementing some for of testing remote functionality based on a given
/// configuration.
pub trait Probe {
    fn execute<'a>(&self) -> Result<ProbeReport>;
    fn identifier(&self) -> String;
}

type ProbeBox = Box<dyn Probe>;
type Probes = Vec<ProbeBox>;
pub(crate) type Data = Vec<(String, String)>;
pub(crate) type ReportsAndErrors = (Vec<ProbeReport>, Vec<InquestError>);

pub type Result<T> = result::Result<T, InquestError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProbeReport {
    pub probe_identifier: String,
    pub data: Data,
}

impl ProbeReport {
    pub(crate) fn new(probe_identifier: String) -> ProbeReport {
        return ProbeReport {
            probe_identifier,
            data: Default::default(),
        };
    }
}

/// We define a ADT (sum type) which we can use for iterating the configured probes.
/// The enum wraps dedicated structs which also implement the Probe trait. By doing so we have a
/// compile-time-check that each probe has the proper configuration. If we would have used an
/// enum-struct there is no way to pass Config::Http as a type (it is  only a variant).
/// The approach taken here combines the best of both worlds.
#[derive(Debug)]
pub(crate) enum Config {
    Http(Http),
    Postgres(Postgres),
    Oracle(Oracle),
    MSSql(MSSql),
}

#[derive(Debug)]
pub(crate) struct GlobalOptions {
    pub(crate) timeout: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct Certificates {
    pub(crate) clientCert: String,
    pub(crate) clientKey: String,
    pub(crate) clientPem: Option<String>,
    pub(crate) caCert: Option<String>,
}

/// Configuration options for a HTTP probe
#[derive(Debug)]
pub(crate) struct Http {
    pub(crate) options: &'static GlobalOptions,
    pub(crate) url: Url,
    pub(crate) status: u16,
    pub(crate) name: Option<String>,
    pub(crate) certs: Option<Certificates>,
}

/// Configuration options for a probe targeting a Postgres database
#[derive(Debug)]
pub(crate) struct Postgres {
    pub(crate) options: &'static GlobalOptions,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) database: String,
    pub(crate) password: SecretString,
    pub(crate) sql: Option<SqlTest>,
    pub(crate) certs: Option<Certificates>,
}

/// Configuration options for a probe targeting a Oracle database
#[derive(Debug)]
pub(crate) struct Oracle {
    pub(crate) options: &'static GlobalOptions,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) sid: String,
    pub(crate) user: String,
    pub(crate) password: SecretString,
    pub(crate) sql: Option<SqlTest>,
}

/// Configuration options for a probe targeting a MSSQL database
#[derive(Debug)]
pub(crate) struct MSSql {
    pub(crate) options: &'static GlobalOptions,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) password: SecretString,
    pub(crate) sql: Option<SqlTest>,
    pub(crate) certs: Option<Certificates>,
}

impl Certificates {
    pub(crate) fn new(
        clientCert: String,
        clientKey: String,
        clientPem: Option<String>,
        caCert: Option<String>,
    ) -> Certificates {
        Certificates {
            clientCert,
            clientKey,
            clientPem,
            caCert,
        }
    }
}

impl Config {
    /// Some Probe configurations will have encrypted secrets when reading the configuration
    /// from an HOCON file. This functions will extract the relevant fields and apply the given
    /// decryption-closure for each, replacing the value in-memory.
    pub(crate) fn decrypt<F>(mut self, decrypt: F) -> Config
    where
        F: Fn(SecretString) -> SecretString,
    {
        if let Config::Postgres(Postgres { password, .. })
        | Config::Oracle(Oracle { password, .. })
        | Config::MSSql(MSSql { password, .. }) = &mut self
        {
            let _old = std::mem::replace(password, decrypt(password.to_owned()));
        }
        self
    }
}

impl From<Oracle> for Config {
    fn from(config: Oracle) -> Self {
        Config::Oracle(config)
    }
}

impl From<Http> for Config {
    fn from(config: Http) -> Self {
        Config::Http(config)
    }
}

impl From<Postgres> for Config {
    fn from(config: Postgres) -> Self {
        Config::Postgres(config)
    }
}

impl From<MSSql> for Config {
    fn from(config: MSSql) -> Self {
        Config::MSSql(config)
    }
}

#[derive(Debug)]
pub struct SqlTest {
    pub(crate) query: String,
}

#[derive(Debug)]
pub struct SqlTestData {}

#[derive(Debug)]
pub struct ServiceSpecification {
    pub(crate) service: String,
    pub(crate) probe_configs: Vec<Config>,
}

fn execute_probes<'a>(probes: Probes) -> Result<ReportsAndErrors> {
    let mut reports = Vec::with_capacity(probes.len());
    let mut failures = Vec::with_capacity(probes.len());
    for probe in probes {
        match probe.execute() {
            Ok(report) => reports.push(report),
            Err(failure) => failures.push(failure),
        }
    }
    Ok((reports, failures))
}

fn prepare_probes_from_spec(specs: Vec<ServiceSpecification>) -> Probes {
    specs
        .into_iter()
        .flat_map(|service| {
            service
                .probe_configs
                .into_iter()
                .map(|config| config.decrypt(|secret| decrypt_secret(secret, None).unwrap()))
                .map(|config| match config {
                    Config::Http(c) => Box::new(c) as ProbeBox,
                    Config::Postgres(c) => Box::new(c) as ProbeBox,
                    Config::Oracle(c) => Box::new(c) as ProbeBox,
                    Config::MSSql(c) => Box::new(c) as ProbeBox,
                })
        })
        .collect()
}

/// Given a path to a HOCON config, the config is parsed, the secrets decrypted, and the probes
/// executed.
pub fn run_from_config(path: &Path) -> Result<ReportsAndErrors> {
    let spec = input::load_hocon_config(path)?;
    let probes = prepare_probes_from_spec(spec);
    execute_probes(probes)
}

#[cfg(test)]
mod tests {
    use crate::error::InquestError::{AssertionMatchingError, FailedExecutionError, IOError};
    use crate::Result;
    use crate::{execute_probes, Probe, ProbeReport};

    #[test]
    fn failed_probe_does_not_cancel_remaining() {
        // GIVEN:
        struct SuccessMock {}
        struct FailedExecutionProbeMock {}
        struct AssertionErrorProbeMock {}

        impl Probe for SuccessMock {
            fn execute<'a>(&self) -> Result<ProbeReport> {
                Ok(ProbeReport {
                    probe_identifier: self.identifier(),
                    data: vec![],
                })
            }

            fn identifier(&self) -> String {
                "success mock".to_string()
            }
        }

        impl Probe for FailedExecutionProbeMock {
            fn execute<'a>(&self) -> Result<ProbeReport> {
                Err(FailedExecutionError {
                    probe_identifier: self.identifier(),
                    source: Box::new(IOError(std::io::Error::from_raw_os_error(1))),
                })
            }

            fn identifier(&self) -> String {
                "failed mock".to_string()
            }
        }

        impl Probe for AssertionErrorProbeMock {
            fn execute<'a>(&self) -> Result<ProbeReport> {
                Err(AssertionMatchingError(
                    "foobar".to_string(),
                    ProbeReport {
                        probe_identifier: self.identifier(),
                        data: vec![],
                    },
                ))
            }

            fn identifier(&self) -> String {
                "assertion error".to_string()
            }
        }

        let probe_failure_execution = Box::new(FailedExecutionProbeMock {});
        let probe_failure_assertion = Box::new(AssertionErrorProbeMock {});
        let probe_success = Box::new(SuccessMock {});

        // WHEN:
        let result = execute_probes(vec![
            probe_failure_execution,
            probe_failure_assertion,
            probe_success,
        ])
        .unwrap();

        // THEN: the result contains 1 successful report in Left/0
        assert_eq!(1, result.0.len());
        assert_matches!(result.0[0], ProbeReport { .. }); // we expect one success-report
                                                          // AND: a FailedExecutionError in Right/1
        assert_matches!(result.1[0], FailedExecutionError { .. });
        // AND: a AssertionError in the Right/1
        assert_matches!(result.1[1], AssertionMatchingError(_, ProbeReport { .. }));
    }
}
