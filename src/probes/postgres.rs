use postgres::{Client, NoTls};
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::AssertionError;
use crate::Result;
use crate::{Data, GlobalOptions, Postgres, Probe, ProbeReport, SqlTest};

const PROBE_NAME: &'static str = "Postgres";

impl Postgres {
    pub fn new(
        host: Option<String>,
        port: Option<u16>,
        database: Option<String>,
        user: String,
        password: SecretString,
        sql: Option<SqlTest>,
        options: &'static GlobalOptions,
    ) -> Postgres {
        Postgres {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(5432),
            user,
            database: database.unwrap_or("postgres".to_string()),
            password,
            sql,
        }
    }
}

/// Implements a Postgres probe based on the postgres crate.
impl Probe for Postgres {
    fn execute(&self) -> Result<ProbeReport> {
        let mut client = Client::configure()
            .host(&self.host)
            .port(self.port)
            .user(&self.user)
            .dbname(&self.database)
            .password(&self.password.expose_secret())
            .connect_timeout(self.options.timeout)
            .connect(NoTls)?;

        let mut report = ProbeReport::new(PROBE_NAME, self.host.clone());

        match foo(&self.sql, &mut client, &mut report) {
            Ok(data) => {
                report.data.extend(data);
                Ok(report)
            }
            Err(e) => Err(e),
        }
    }
}

fn foo(sql: &Option<SqlTest>, client: &mut Client, report: &mut ProbeReport) -> Result<Data> {
    match sql {
        None => Ok(Default::default()),
        Some(sql) => {
            let query_result = client.query(sql.query.as_str(), &[]);
            match query_result {
                Ok(rows) => {
                    let data: Data = rows
                        .into_iter()
                        .enumerate()
                        .map(|(pos, row)| (pos.to_string(), format!("{:?}", row)))
                        .collect();
                    Ok(data)
                }
                Err(_) => Err(AssertionError(report.clone())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Postgres, GO};
    use secrecy::SecretString;
    use std::str::FromStr;

    #[test]
    fn probe_uses_documented_defaults() {
        let probe = Postgres::new(
            None,
            None,
            None,
            "user".to_string(),
            SecretString::from_str("password").unwrap(),
            None,
            &GO,
        );

        assert_eq!("localhost", &probe.host);
        assert_eq!(5432, probe.port);
        assert_eq!("postgres", &probe.database);
    }
}
