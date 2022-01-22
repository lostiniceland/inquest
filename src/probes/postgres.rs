use postgres::{Client, NoTls};
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::sql::Table;
use crate::Result;
use crate::{Data, GlobalOptions, Postgres, Probe, ProbeReport, SqlTest};
use chrono::Utc;
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
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
        /// The postgres-crate does not provide a default mapping to fallback to String for all
        /// types: row.get is generic and without a type assignment the FromSql-Trait cannot be inferred.
        /// This function matches over the current column-type and does a manual conversion
        fn reflective_get(row: &postgres::Row, index: usize) -> String {
            let column_type = row.columns().get(index).map(|c| c.type_().name()).unwrap();
            // see https://docs.rs/postgres/latest/postgres/types/trait.ToSql.html
            let value = match column_type {
                "bool" => {
                    let v: Option<bool> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "varchar" | "char(n)" | "text" | "name" => {
                    let v: Option<String> = row.get(index);
                    v
                }
                "char" | "bpchar" => {
                    let v: Option<String> = row.get(index);
                    v
                }
                "int2" | "smallserial" | "smallint" => {
                    let v: Option<i16> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "int" | "int4" | "serial" => {
                    let v: Option<i32> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "int8" | "bigserial" | "bigint" => {
                    let v: Option<i64> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "float4" | "real" => {
                    let v: Option<f32> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "float8" | "double precision" => {
                    let v: Option<f64> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "timestamp" => {
                    // with-chrono feature is needed for this
                    let v: Option<chrono::NaiveDateTime> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "timestamptz" | "timestamp with time zone" => {
                    // with-chrono feature is needed for this
                    let v: Option<chrono::DateTime<Utc>> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "time" => {
                    // with-time feature is needed for this
                    let v: Option<time::Time> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "date" => {
                    // with-time feature is needed for this
                    let v: Option<time::Date> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "bit" | "varbit" => {
                    // with-bit-vec feature is needed for this
                    let v: Option<bit_vec::BitVec> = row.get(index);
                    v.map(|v| {
                        v.iter().enumerate().fold(String::new(), |a, b| {
                            a + format!("Bit {}: {}; ", b.0 + 1, b.1).as_str()
                        })
                    })
                }
                "uuid" => {
                    // with-uuid feature is needed for this
                    let v: Option<uuid::Uuid> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "inet" => {
                    // with-eui48 feature is needed for this
                    let v: Option<IpAddr> = row.get(index);
                    v.map(|v| v.to_string())
                }
                "macaddr" => {
                    // with-eui48 feature is needed for this
                    let v: Option<eui48::MacAddress> = row.get(index);
                    v.map(|v| v.to_string(eui48::MacAddressFormat::Canonical))
                }
                "point" => {
                    // with-geo feature is needed for this
                    let v: Option<geo_types::Point<f64>> = row.get(index);
                    v.map(|v| format!("x={} y={}", v.0.x, v.0.y))
                }
                "box" => {
                    // with-geo feature is needed for this
                    let v: Option<geo_types::Rect<f64>> = row.get(index);
                    v.map(|v| {
                        format!(
                            "x1={} y1={} x2={} y2={}",
                            v.min().x,
                            v.min().y,
                            v.max().x,
                            v.max().y
                        )
                    })
                }
                "path" => {
                    // with-geo feature is needed for this
                    let v: Option<geo_types::LineString<f64>> = row.get(index);
                    v.map(|v| {
                        v.into_iter()
                            .map(|coord| format!("x={} y={}", coord.x, coord.y))
                            .enumerate()
                            .fold(String::new(), |a, b| {
                                a + format!("Coordinate {}: {}; ", b.0 + 1, b.1).as_str()
                            })
                    })
                }
                "json" => {
                    // with-serde-json feature is needed for this
                    let v: Option<serde_json::Value> = row.get(index);
                    v.map(|v| v.to_string())
                }
                &_ => Some(format!("CANNOT PARSE '{}'", column_type)),
            };
            value.unwrap_or("".to_string())
        }

        let columns = item
            .first()
            .map(|row| row.columns())
            .unwrap_or(&[])
            .iter()
            .map(|col| col.name().to_string())
            .collect();
        let rows = item
            .into_iter()
            .map(|row| {
                let range = 0..row.len();
                let mut table_row = Vec::with_capacity(range.len());
                // FIXME deserialization not working
                for index in range.step_by(1) {
                    // let x: core::result::Result<Option<&str>, tokio_postgres::Error> =
                    //     row.try_get(index);
                    //
                    // if let Ok(Some(value)) = x {
                    //     table_row.push(value.to_string());
                    // } else {
                    //     table_row.push("XXX".to_string());
                    // }
                    // let x: Vec<u8> = row.get(index);
                    // table_row.push(String::from_utf8_lossy(x.as_slice()).to_string());
                    table_row.push(reflective_get(&row, index));
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
