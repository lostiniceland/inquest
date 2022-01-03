use postgres::{Client, NoTls};
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::sql::Table;
use crate::Result;
use crate::{Data, GlobalOptions, Postgres, Probe, ProbeReport, SqlTest};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::vec;

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
        let mut client = establish_connection(self)?;
        // connection was successful
        let mut report = ProbeReport::new(self.identifier());

        match run_sql(&self, &mut client, &mut report) {
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
            PROBE_NAME, self.host, self.port, self.database, self.user
        )
    }
}

fn establish_connection(probe: &Postgres) -> Result<Client> {
    Client::configure()
        .host(&probe.host)
        .port(probe.port)
        .user(&probe.user)
        .dbname(&probe.database)
        .password(&probe.password.expose_secret())
        .connect_timeout(probe.options.timeout)
        .connect(NoTls)
        .map_err(|e| FailedExecutionError {
            probe_identifier: probe.identifier(),
            source: Box::new(e),
        })
}

fn run_sql(probe: &Postgres, client: &mut Client, _: &mut ProbeReport) -> Result<Data> {
    match &probe.sql {
        None => Ok(Default::default()),
        Some(sql) => {
            let query_result = client.query(sql.query.as_str(), &[]);
            match query_result {
                Ok(rows) => {
                    let data: Data =
                        vec![("ResultSet".to_string(), format!("{}", Table::from(rows)))];
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

impl<'set> From<Vec<postgres::Row>> for Table {
    fn from(item: Vec<postgres::Row>) -> Self {
        let columns = item
            .first()
            .map(|row| row.columns())
            .unwrap_or(&[])
            .iter()
            .map(|col| col.name().to_string())
            .collect();
        let rows = item
            .iter()
            .map(|row| {
                let range = 0..row.len();
                let mut table_row = Vec::with_capacity(range.len());
                // FIXME deserialization not working
                for index in range.step_by(1) {
                    let x: core::result::Result<Option<&str>, tokio_postgres::Error> =
                        row.try_get(index);

                    if let Ok(Some(value)) = x {
                        table_row.push(value.to_string());
                    } else {
                        // deseriali
                        table_row.push("XXX".to_string());
                    }
                }
                table_row
            })
            .collect();
        Table::new(columns, rows)
    }
}

impl ToSocketAddrs for Postgres {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        format!("{}:{}", self.host, self.port).to_socket_addrs()
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
