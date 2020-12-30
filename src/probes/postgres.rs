use postgres::{Client, NoTls, Error, Row};
use secrecy::{ExposeSecret, SecretString};

use crate::core::{Postgres, Probe, ProbeReport, SqlTest, GlobalOptions, Data, Metrics};
use crate::core::Result;
use std::collections::HashMap;
use crate::core::error::InquestError::AssertionError;

const PROBE_NAME: &'static str = "Postgres";

impl Postgres {
    pub fn new(host: Option<String>, port: Option<u16>, database: Option<String>, user: String, password: SecretString, sql: Option<SqlTest>, options: &'static GlobalOptions) -> Postgres {
        Postgres {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(5432),
            user,
            database: database.unwrap_or("postgres".to_string()),
            password,
            sql
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
            .connect_timeout(self.options.timeout).connect(NoTls)?;

        let mut report = ProbeReport {
            probe_name: PROBE_NAME,
            probe_identifier: self.host.clone(),
            data: Default::default(),
            metrics: Default::default()
        };

        match foo(&self.sql, &mut client, &mut report){
            Ok((data, metrics)) => {
                report.data.extend(data);
                report.metrics.extend(metrics);
                Ok(report)
            }
            Err(e) => Err(e)
        }
    }
}

fn foo(sql: &Option<SqlTest>, client: &mut Client, report: &ProbeReport) -> Result<(Data, Metrics)> {
    match sql {
        None => Ok((Default::default(), Default::default())),
        Some(sql) => {
            let query_result = client.query(sql.query.as_str(), &[]);
            match query_result {
                Ok(rows) => {
                    let data: Data = rows.into_iter().enumerate().map(|(pos, row)| (pos.to_string(), format!("{:?}", row))).collect();
                    let mut metrics = Vec::with_capacity(1);
                    metrics.sort();
                    Ok((data, metrics))
                }
                Err(_) => Err(AssertionError(report.clone()))
            }
        }
    }
}
