use secrecy::{ExposeSecret, SecretString};
use tiberius::error::Error;
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::sql::Table;
use crate::Result;
use crate::{Data, GlobalOptions, MSSql, Probe, ProbeReport, SqlTest};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::vec;

const PROBE_NAME: &'static str = "MSSql";

impl MSSql {
    pub fn new(
        host: Option<String>,
        port: Option<u16>,
        user: String,
        password: SecretString,
        sql: Option<SqlTest>,
        options: &'static GlobalOptions,
    ) -> MSSql {
        MSSql {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(1433),
            user,
            password,
            sql,
        }
    }
}

/// Implements a MSSql probe based on the MSSql crate.
impl Probe for MSSql {
    fn execute(&self) -> Result<ProbeReport> {
        let future = async {
            match establish_connection(self, None, None).await {
                Ok(con) => Ok(con),
                Err(Error::Routing { host, port }) => {
                    establish_connection(self, Some(host), Some(port)).await
                }
                Err(e) => Err(e),
            }
        };
        let tokio_runtime = Runtime::new().unwrap();
        let mut client = tokio_runtime
            .block_on(future)
            .map_err(|e| FailedExecutionError {
                probe_identifier: self.identifier(),
                source: Box::new(e),
            })?;
        let mut report = ProbeReport::new(self.identifier());

        tokio_runtime.block_on(run_sql(&self, &mut client, &mut report))?;
        Ok(report)
    }

    fn identifier(&self) -> String {
        format!("{} - {}:{}/{}", PROBE_NAME, self.host, self.port, self.user)
    }
}

/// Creates a future yielding the MSSQL client.
/// In case a Error::Routing is received, this method is called again with the updated host and port
async fn establish_connection(
    probe: &MSSql,
    redirect_host: Option<String>,
    redirect_port: Option<u16>,
) -> std::result::Result<Client<Compat<TcpStream>>, tiberius::error::Error> {
    let mut config = Config::new();
    config.trust_cert();
    config.authentication(AuthMethod::sql_server(
        &probe.user,
        &probe.password.expose_secret(),
    ));
    // in case we have receive a redirect-error on the first attempt, use the redirect-options
    // instead of the probe-values
    config.host(redirect_host.unwrap_or(probe.host.clone()));
    config.port(redirect_port.unwrap_or(probe.port));

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    // we should not have more than one redirect, so we'll short-circuit here.
    Client::connect(config, tcp.compat_write()).await
}

async fn run_sql(
    probe: &MSSql,
    client: &mut Client<Compat<TcpStream>>,
    report: &mut ProbeReport,
) -> Result<()> {
    match &probe.sql {
        None => Ok(Default::default()),
        Some(sql) => {
            match client
                .simple_query(sql.query.as_str())
                .await
                .map_err(|e| FailedAssertionError {
                    probe_identifier: probe.identifier(),
                    desc: "Error execution sql-query!".to_string(),
                    source: Box::new(e),
                })?
                .into_results()
                .await
            {
                Ok(rows) => {
                    let data: Data = vec![(
                        "ResultSet".to_string(),
                        format!(
                            "{}",
                            Table::from(rows.first().unwrap_or(Vec::with_capacity(0).as_ref()))
                        ),
                    )];
                    report.data.extend(data);
                    Ok(())
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

impl<'set> From<&Vec<tiberius::Row>> for Table {
    fn from(item: &Vec<tiberius::Row>) -> Self {
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
                // FIXME desierialization not working
                for index in range.step_by(1) {
                    let x: tiberius::Result<Option<&str>> = row.try_get(index);

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

// impl FromSql for String {
//     fn from_sql(value: &'a ColumnData<'static>) -> tiberius::Result<Option<Self>> {
//         let string_value = match value {
//             ColumnData::U8(data) => { data.map(|&raw| String::from_utf8_lossy(raw).to_string()).unwrap_or("".to_string())}
//             ColumnData::I16(data) => { data.map(|number| String::from(number)).unwrap_or("".to_string())}
//             ColumnData::I32(data) => {data.map(|number| String::from(number)).unwrap_or("".to_string())}
//             ColumnData::I64(data) => {data.map(|number| String::from(number)).unwrap_or("".to_string())}
//             ColumnData::F32(data) => {data.map(|number| String::from(number)).unwrap_or("".to_string())}
//             ColumnData::F64(data) => {data.map(|number| String::from(number)).unwrap_or("".to_string())}
//             ColumnData::Bit(data) => {}
//             ColumnData::String(_) => {"".to_string()}
//             ColumnData::Guid(_) => {"".to_string()}
//             ColumnData::Binary(_) => {"".to_string()}
//             ColumnData::Numeric(_) => {"".to_string()}
//             ColumnData::Xml(_) => {"".to_string()}
//             ColumnData::DateTime(_) => {"".to_string()}
//             ColumnData::SmallDateTime(_) => {"".to_string()}
//             ColumnData::Time(_) => {"".to_string()}
//             ColumnData::Date(_) => {"".to_string()}
//             ColumnData::DateTime2(_) => {"".to_string()}
//             ColumnData::DateTimeOffset(_) => {"".to_string()}
//         };
//         Ok(Some(string_value))
//     }
// }

impl ToSocketAddrs for MSSql {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        format!("{}:{}", self.host, self.port).to_socket_addrs()
    }
}

#[cfg(test)]
mod tests {
    use crate::{MSSql, GO};
    use secrecy::SecretString;
    use std::str::FromStr;

    #[test]
    fn probe_uses_documented_defaults() {
        let probe = MSSql::new(
            None,
            None,
            "user".to_string(),
            SecretString::from_str("password").unwrap(),
            None,
            &GO,
        );

        assert_eq!("localhost", &probe.host);
        assert_eq!(1433, probe.port);
    }
}
