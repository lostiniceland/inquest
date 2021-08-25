use oracle::Connection;
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::{AssertionError, FailedExecutionError};
use crate::Result;
use crate::{Data, GlobalOptions, Oracle, Probe, ProbeReport, SqlTest};

// const GO_REMOVE: GlobalOptions = GlobalOptions { timeout: Duration::from_secs(30) };

const PROBE_NAME: &'static str = "Oracle";

impl Oracle {
    pub(crate) fn new(
        host: Option<String>,
        port: Option<u16>,
        sid: String,
        user: String,
        password: SecretString,
        sql: Option<SqlTest>,
        options: &'static GlobalOptions,
    ) -> Oracle {
        Oracle {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(1521),
            sid,
            user,
            password,
            sql,
        }
    }
}

/// Implements a Oracle probe based on the oracle crate which again uses the ODPI-C client.
impl Probe for Oracle {
    fn execute(&self) -> Result<ProbeReport> {
        let connection = establish_connection(self)?;
        let mut report = ProbeReport::new(
            PROBE_NAME,
            format!("{}:{}/{}/{}", self.host, self.port, self.sid, self.user),
        );

        match run_sql(&self.sql, &connection, &mut report) {
            Ok(data) => {
                report.data.extend(data);
                Ok(report)
            }
            Err(e) => Err(e),
        }
    }
}

fn establish_connection(probe: &Oracle) -> Result<Connection> {
    let connection_string = format!("//{}:{}/{}", &probe.host, &probe.port, &probe.sid);
    Connection::connect(
        &probe.user,
        &probe.password.expose_secret(),
        connection_string.clone(),
    )
    // FIXME why is this mapping needed? There is a FROM in errors.rs
    .map_err(|e| FailedExecutionError {
        source: Box::new(e),
    })
}

fn run_sql(sql: &Option<SqlTest>, connection: &Connection, report: &ProbeReport) -> Result<Data> {
    match sql {
        None => Ok(Default::default()),
        Some(sql) => {
            let query_result = connection.query(&sql.query, &[]);
            match query_result {
                Ok(rows) => {
                    let data: Data = rows
                        .enumerate()
                        .map(|(pos, row)| (pos.to_string(), format!("{:?}", row.unwrap())))
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
    use crate::{Oracle, GO};
    use secrecy::SecretString;
    use std::str::FromStr;

    #[test]
    fn probe_uses_documented_defaults() {
        let probe = Oracle::new(
            None,
            None,
            "SID".to_string(),
            "user".to_string(),
            SecretString::from_str("password").unwrap(),
            None,
            &GO,
        );

        assert_eq!("localhost", &probe.host);
        assert_eq!(1521, probe.port);
    }
}
