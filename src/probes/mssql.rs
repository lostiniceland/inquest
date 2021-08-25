use secrecy::{ExposeSecret, SecretString};
use tiberius::error::Error;
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::error::InquestError::AssertionError;
use crate::Result;
use crate::{Data, GlobalOptions, MSSql, Probe, ProbeReport, SqlTest};

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
        let mut client = tokio_runtime.block_on(future)?;
        let mut report = ProbeReport::new(
            PROBE_NAME,
            format!("{}:{}/{}", self.host, self.port, self.user),
        );

        tokio_runtime.block_on(run_sql(&self.sql, &mut client, &mut report))?;
        Ok(report)
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
    sql: &Option<SqlTest>,
    client: &mut Client<Compat<TcpStream>>,
    report: &mut ProbeReport,
) -> Result<()> {
    match sql {
        None => Ok(Default::default()),
        Some(sql) => {
            match client
                .simple_query(sql.query.as_str())
                .await?
                .into_results()
                .await
            {
                Ok(rows) => {
                    let data: Data = rows
                        .into_iter()
                        .enumerate()
                        .map(|(pos, row)| (pos.to_string(), format!("{:?}", row)))
                        .collect();
                    report.data.extend(data);
                    Ok(())
                }
                Err(_) => Err(AssertionError(report.clone())),
            }
        }
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
