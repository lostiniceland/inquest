use secrecy::{ExposeSecret, SecretString};
use tiberius::error::Error;
use tiberius::{AuthMethod, Client, ColumnData, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::sql::Table;
use crate::{Certificates, Result};
use crate::{Data, GlobalOptions, MSSql, Probe, ProbeReport, SqlTest};
use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};
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
        certs: Option<Certificates>,
    ) -> MSSql {
        MSSql {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(1433),
            user,
            password,
            sql,
            certs,
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

        tokio_runtime.block_on(run_sql(self, &mut client, &mut report))?;
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
    // config.trust_cert();
    config.authentication(AuthMethod::sql_server(
        &probe.user,
        probe.password.expose_secret(),
    ));
    if let Some(cert_options) = &probe.certs {
        if let Some(ca_cert_path) = &cert_options.ca_cert {
            config.trust_cert_ca(ca_cert_path);
        }

        if cert_options.client_cert.is_some()
            || cert_options.client_key.is_some()
            || cert_options.client_pem.is_some()
        {
            // TODO use proper logging/eventing
            println!("MTLS currently not supported by MSSQL driver");
        }
    }

    // in case we have receive a redirect-error on the first attempt, use the redirect-options
    // instead of the probe-values
    config.host(redirect_host.unwrap_or(probe.host.clone()));
    config.port(redirect_port.unwrap_or(probe.port));

    let tcp = TcpStream::connect(config.get_addr()).await?;
    // let stream = async_native_tls::connect(config.get_addr(), tcp).await?;
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
                Ok(mut rows) => {
                    let data: Data = vec![(
                        "ResultSet".to_string(),
                        format!("{}", Table::from(rows.remove(0))), // first vec is the query (could be multiple)
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

impl<'set> From<Vec<tiberius::Row>> for Table {
    fn from(item: Vec<tiberius::Row>) -> Self {
        // inlines copied from tiberius::tds:time::chrono
        #[inline]
        fn from_days(days: i64, start_year: i32) -> chrono::NaiveDate {
            chrono::NaiveDate::from_ymd_opt(start_year, 1, 1).unwrap()
                + chrono::Duration::days(days)
        }

        #[inline]
        fn from_sec_fragments(sec_fragments: i64) -> chrono::NaiveTime {
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                + chrono::Duration::nanoseconds(sec_fragments * (1e9 as i64) / 300)
        }

        #[inline]
        fn from_mins(mins: u32) -> chrono::NaiveTime {
            chrono::NaiveTime::from_num_seconds_from_midnight_opt(mins, 0).unwrap()
        }

        // #[inline]
        // fn to_days(date: chrono::NaiveDate, start_year: i32) -> i64 {
        //     date.signed_duration_since(chrono::NaiveDate::from_ymd(start_year, 1, 1))
        //         .num_days()
        // }

        let columns = item
            .first()
            .map(|row| row.columns())
            .unwrap_or(&[])
            .iter()
            .map(|col| col.name().to_string())
            .collect();
        let mut my_rows = Vec::with_capacity(item.len());

        for row in item {
            let mut table_row = Vec::with_capacity(20);

            for item in row.into_iter() {
                let value = match item {
                    ColumnData::Binary(_val) => "<binary data>".into(),
                    ColumnData::Bit(val) => val.unwrap_or_default().to_string(),
                    ColumnData::F32(val) => val.unwrap_or_default().to_string(),
                    ColumnData::F64(val) => val.unwrap_or_default().to_string(),
                    ColumnData::Guid(val) => val.unwrap_or_default().to_string(),
                    ColumnData::I16(val) => val.unwrap_or_default().to_string(),
                    ColumnData::I32(val) => val.unwrap_or_default().to_string(),
                    ColumnData::I64(val) => val.unwrap_or_default().to_string(),
                    ColumnData::Numeric(val) => val.unwrap().to_string(),
                    ColumnData::String(val) => val.unwrap_or_default().as_ref().into(),
                    ColumnData::U8(val) => val.unwrap_or_default().to_string(),
                    ColumnData::Xml(val) => val.unwrap().as_ref().to_string(),
                    ColumnData::Date(ref val) => val
                        .map(|date| {
                            let date = from_days(date.days() as i64, 1);
                            date.format("%Y-%m-%d").to_string()
                        })
                        .unwrap_or_default(),
                    ColumnData::DateTime(ref val) => val
                        .map(|dt| {
                            let datetime = NaiveDateTime::new(
                                from_days(dt.days() as i64, 1900),
                                from_sec_fragments(dt.seconds_fragments() as i64),
                            );
                            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                    ColumnData::DateTime2(ref val) => val
                        .map(|dt| {
                            let datetime = NaiveDateTime::new(
                                from_days(dt.date().days() as i64, 1),
                                NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                                    + chrono::Duration::nanoseconds(
                                        dt.time().increments() as i64
                                            * 10i64.pow(9 - dt.time().scale() as u32),
                                    ),
                            );
                            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                    ColumnData::DateTimeOffset(ref val) => val
                        .map(|dto| {
                            let date = from_days(dto.datetime2().date().days() as i64, 1);
                            let ns = dto.datetime2().time().increments() as i64
                                * 10i64.pow(9 - dto.datetime2().time().scale() as u32);

                            let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                                + chrono::Duration::nanoseconds(ns)
                                - chrono::Duration::minutes(dto.offset() as i64);
                            let naive = NaiveDateTime::new(date, time);

                            let dto: DateTime<Utc> = chrono::DateTime::from_utc(naive, Utc);
                            dto.format("%Y-%m-%d %H:%M:%S %z").to_string()
                        })
                        .unwrap_or_default(),
                    ColumnData::SmallDateTime(ref val) => val
                        .map(|dt| {
                            let datetime = NaiveDateTime::new(
                                from_days(dt.days() as i64, 1900),
                                from_mins(dt.seconds_fragments() as u32 * 60),
                            );
                            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                    ColumnData::Time(ref val) => val
                        .map(|time| {
                            let ns = time.increments() as i64 * 10i64.pow(9 - time.scale() as u32);
                            let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                                + chrono::Duration::nanoseconds(ns);
                            format!("{}", time.format("%H:%M:%S"))
                        })
                        .unwrap_or_default(),
                };
                table_row.push(value);
            }
            my_rows.push(table_row);
        }
        Table::new(columns, my_rows)
    }
}

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
            None,
        );

        assert_eq!("localhost", &probe.host);
        assert_eq!(1433, probe.port);
    }
}
