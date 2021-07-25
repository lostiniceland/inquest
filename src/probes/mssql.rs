use secrecy::{ExposeSecret, SecretString};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
// use async_std::net::TcpStream;
// use async_std::future;
use tokio::runtime::Runtime;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::core::{Data, GlobalOptions, MSSql, Probe, ProbeReport, SqlTest};
use crate::core::Result;

const PROBE_NAME: &'static str = "MSSql";

impl MSSql {
    pub fn new(host: Option<String>, port: Option<u16>, user: String, password: SecretString, sql: Option<SqlTest>, options: &'static GlobalOptions) -> MSSql {
        MSSql {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(5432),
            user,
            password,
            sql
        }
    }
}

/// Implements a MSSql probe based on the MSSql crate.
impl Probe for MSSql {
    fn execute(&self) -> Result<ProbeReport> {

        let mut config = Config::new();

        config.host(&self.host);
        config.port(self.port);
        config.trust_cert();
        config.authentication(AuthMethod::sql_server(&self.user, &self.password.expose_secret()));

        // To be able to use Tokio's tcp, we're using the `compat_write` from
        // the `TokioAsyncWriteCompatExt` to get a stream compatible with the
        // traits from the `futures` crate.
        let report_future = async {
            let tcp = TcpStream::connect(config.get_addr()).await?;
            tcp.set_nodelay(true)?;
            let mut client = Client::connect(config, tcp.compat_write()).await?;
            let rows = match &self.sql {
                None => Default::default(),
                Some(sql) => {
                    let query_stream = client.simple_query(sql.query.as_str()).await?;
                    // we expect only simple queries, so we do not stream each row and rather collect the full set into memory
                    let rows = query_stream.into_results().await?;
                    Some(rows)
                }
            };

            match rows {
                None => Ok((Default::default(), Default::default())),
                Some(rows) => {
                    let data: Data = rows.into_iter().flatten().enumerate().map(|(pos, row)| (pos.to_string(), format!("{:?}", row))).collect();
                    println!("{:?}", data);
                    let metrics = Vec::with_capacity(1);
                    Ok((data, metrics))
                }
            }

        };
        let result = Runtime::new().unwrap().block_on(report_future);

        let mut report = ProbeReport {
            probe_name: PROBE_NAME,
            probe_identifier: self.host.clone(),
            data: Default::default(),
            metrics: Default::default()
        };

        match result {
            Ok((data, metrics)) => {
                report.data.extend(data);
                report.metrics.extend(metrics);
                Ok(report)
            }
            Err(e) => Err(e)
        }
    }
}

// async fn foo(sql: &Option<SqlTest>, client: &mut Client<Compat<TcpStream>>,report: &ProbeReport) -> Result<(Data, Metrics)> {
//     match sql {
//         None => Ok((Default::default(), Default::default())),
//         Some(sql) => {
//             let query_result = client.simple_query(sql.query.as_str());
//             match query_result {
//                 Ok(rows) => {
//                     let data: Data = rows.into_iter().enumerate().map(|(pos, row)| (pos.to_string(), format!("{:?}", row))).collect();
//                     let mut metrics = Vec::with_capacity(1);
//                     metrics.sort();
//                     Ok((data, metrics))
//                 }
//                 Err(_) => Err(AssertionError(report.clone()))
//             }
//         }
//     }
// }
