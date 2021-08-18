
use oracle::Connection;
use secrecy::{ExposeSecret, SecretString};

use crate::{Data, GlobalOptions, Metrics, Oracle, Probe, ProbeReport, SqlTest};
use crate::error::InquestError::AssertionError;
use crate::Result;

// const GO_REMOVE: GlobalOptions = GlobalOptions { timeout: Duration::from_secs(30) };

const PROBE_NAME: &'static str = "Oracle";

impl Oracle {
    pub(crate) fn new(host: Option<String>, port: Option<u16>, sid: String, user: String, password: SecretString, sql: Option<SqlTest>, options: &'static GlobalOptions) -> Oracle {
        Oracle{
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(1521),
            sid,
            user,
            password,
            sql
        }
    }
}

/// Implements a Oracle probe based on the oracle crate which again uses the ODPI-C client.
impl Probe for Oracle {
    fn execute(&self) -> Result<ProbeReport> {
        let connection_string = format!(
            "//{}:{}/{}",
            &self.host,
            &self.port,
            &self.sid
        );
        let connection = Connection::connect(
            &self.user,
            &self.password.expose_secret(),
            connection_string.clone(),
        )?;

        let mut report = ProbeReport {
            probe_name: PROBE_NAME,
            probe_identifier: connection_string,
            data: Default::default(),
            metrics: Default::default()
        };

        match foo(&self.sql, &connection, &mut report){
            Ok((data, metrics)) => {
                report.data.extend(data);
                report.metrics.extend(metrics);
                Ok(report)
            }
            Err(e) => Err(e)
        }
    }
}

fn foo(sql: &Option<SqlTest>, connection: &Connection, report: &ProbeReport) -> Result<(Data, Metrics)> {
    match sql {
        None => Ok((Default::default(), Default::default())),
        Some(sql) => {
            let query_result = connection.query(&sql.query, &[]);
            match query_result {
                Ok(rows) => {
                    let data: Data = rows.enumerate().map(|(pos, row)| (pos.to_string(), format!("{:?}", row.unwrap()))).collect();
                    let mut metrics = Vec::with_capacity(1);
                    metrics.sort();
                    Ok((data, metrics))
                }
                Err(_) => Err(AssertionError(report.clone()))
            }
        }
    }
}