use oracle::Connection;
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::tcp::foo;
use crate::Result;
use crate::{Data, GlobalOptions, Oracle, Probe, ProbeReport, SqlTest};
use std::net::{SocketAddr, ToSocketAddrs};
use std::{io, vec};

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
        let mut report = ProbeReport::new(self.identifier());

        match run_sql(&self, &connection, &mut report) {
            Ok(data) => {
                report.data.extend(data);
                Ok(report)
            }
            Err(e) => Err(e),
        }
    }

    fn identifier(&self) -> String {
        format!(
            "{} - {}:{}/{}/{}",
            PROBE_NAME, self.host, self.port, self.sid, self.user
        )
    }
}

fn establish_connection(probe: &Oracle) -> Result<Connection> {
    let connection_string = format!("//{}:{}/{}", &probe.host, &probe.port, &probe.sid);
    let r = Connection::connect(
        &probe.user,
        &probe.password.expose_secret(),
        connection_string.clone(),
    )
    // FIXME why is this mapping needed? There is a FROM in errors.rs
    .map_err(|e| FailedExecutionError {
        probe_identifier: probe.identifier(),
        source: Box::new(e),
    });
    if r.is_err() {
        foo(probe)
    }
    r
}

fn run_sql(probe: &Oracle, connection: &Connection, report: &ProbeReport) -> Result<Data> {
    match &probe.sql {
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
                Err(e) => Err(FailedAssertionError {
                    probe_identifier: probe.identifier(),
                    desc: "Error execution sql-query!".to_string(),
                    source: Box::new(e),
                }),
            }
        }
    }
}

impl ToSocketAddrs for Oracle {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        format!("{}:{}", self.host, self.port).to_socket_addrs()
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
