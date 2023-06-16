use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError::{
    AssertionMatchingError, FailedAssertionError, FailedExecutionError,
};
use crate::probes::sql::Table;
use crate::{Certificates, Result};
use crate::{Data, GlobalOptions, Postgres, Probe, ProbeReport, SqlTest};
use chrono::Utc;
use rustls::RootCertStore;
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::vec;
use tokio::runtime::Runtime;
use tokio_postgres::{Client, Config, Connection, Socket};
use tokio_postgres_rustls::RustlsStream;

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
        certs: Option<Certificates>,
    ) -> Postgres {
        Postgres {
            options,
            host: host.unwrap_or("localhost".to_string()),
            port: port.unwrap_or(5432),
            user,
            database: database.unwrap_or("postgres".to_string()),
            password,
            sql,
            certs,
        }
    }
}

/// Implements a Postgres probe based on the postgres crate.
impl Probe for Postgres {
    fn execute(&self) -> Result<ProbeReport> {
        let future = async {
            match establish_connection(self).await {
                Ok((client, con)) => Ok((client, con)),
                Err(e) => Err(e),
            }
        };
        let tokio_runtime = Runtime::new().unwrap();
        let mut client_con = tokio_runtime
            .block_on(future)
            .map_err(|e| FailedExecutionError {
                probe_identifier: self.identifier(),
                source: Box::new(e),
            })?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        let handle = tokio_runtime.spawn(async move {
            if let Err(e) = client_con.1.await {
                eprintln!("connection error: {}", e);
            }
        });

        // connection was successful
        let mut report = ProbeReport::new(self.identifier());

        tokio_runtime.block_on(run_sql(&self, &mut client_con.0, &mut report))?;
        handle.abort(); // kill the connection-thread
        Ok(report)
    }

    fn identifier(&self) -> String {
        format!(
            "{} - {}:{}/{}/{}",
            PROBE_NAME, self.host, self.port, self.database, self.user
        )
    }
}

async fn establish_connection(
    probe: &Postgres,
) -> Result<(Client, Connection<Socket, RustlsStream<Socket>>)> {
    let mut root_store = rustls::RootCertStore::empty();
    // root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
    //     rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
    //         ta.subject,
    //         ta.spki,
    //         ta.name_constraints,
    //     )
    // }));

    for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs") {
        root_store.add(&rustls::Certificate(cert.0)).unwrap();
    }

    let tls_client_config = if let Some(cert_options) = &probe.certs {
        if let Some(ca_cert_path) = &cert_options.ca_cert {
            let ca_certs = certs::load_certificate_chain(ca_cert_path, probe)?;
            ca_certs
                .iter()
                .try_for_each(|ca| root_store.add(ca))?
                // .map_err(|e| FailedExecutionError {
                //     probe_identifier: probe.identifier(),
                //     source: Box::new(e),
                // })?
                ;
        }

        if let (Some(client_key), Some(client_cert)) =
            (&cert_options.client_key, &cert_options.client_cert)
        {
            let certs = certs::load_certificate_chain(client_cert, probe)?;
            let private_key = certs::load_private_key(client_key, probe)?;
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_single_cert(certs, private_key)?
            // .map_err(|e| FailedExecutionError {
            //     probe_identifier: probe.identifier(),
            //     source: Box::new(e),
            // })?
        } else {
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        }
    } else {
        rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth()
    };

    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_client_config);
    Config::new()
        .host(&probe.host)
        .port(probe.port)
        .user(&probe.user)
        .dbname(&probe.database)
        .password(probe.password.expose_secret())
        .connect_timeout(probe.options.timeout)
        .connect(tls)
        .await
        .map_err(|e| FailedExecutionError {
            probe_identifier: probe.identifier(),
            source: Box::new(e),
        })
}

async fn run_sql(probe: &Postgres, client: &mut Client, report: &mut ProbeReport) -> Result<()> {
    match &probe.sql {
        None => Ok(Default::default()),
        Some(sql) => {
            println!("is client closed: {}", client.is_closed());
            let query_result = client.query(sql.query.as_str(), &[]).await;
            match query_result {
                Ok(rows) => {
                    let data: Data =
                        vec![("ResultSet".to_string(), format!("{}", Table::from(rows)))];
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
                for index in range.step_by(1) {
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

mod certs {
    use crate::InquestError::FailedExecutionError;
    use crate::Result;
    use crate::{InquestError, Probe};
    use std::{fs, io};

    pub(crate) fn load_certificate_chain(
        ca_cert_path: &String,
        probe: &dyn Probe,
    ) -> Result<Vec<rustls::Certificate>> {
        // Open certificate file.
        let certfile = fs::File::open(ca_cert_path).map_err(|e| FailedExecutionError {
            probe_identifier: probe.identifier(),
            source: Box::new(e),
        })?;
        let mut reader = io::BufReader::new(certfile);

        // Load and return certificate.
        let certs = rustls_pemfile::certs(&mut reader).map_err(|e| FailedExecutionError {
            probe_identifier: probe.identifier(),
            source: Box::new(e),
        })?;

        Ok(certs.into_iter().map(rustls::Certificate).collect())
    }

    pub(crate) fn load_private_key(
        filename: &str,
        probe: &dyn Probe,
    ) -> Result<rustls::PrivateKey> {
        // Open keyfile.
        let keyfile = fs::File::open(filename).map_err(|e| FailedExecutionError {
            probe_identifier: probe.identifier(),
            source: Box::new(e),
        })?;
        let mut reader = io::BufReader::new(keyfile);
        // Load and return a single private key.
        let keys =
            rustls_pemfile::pkcs8_private_keys(&mut reader).map_err(|e| FailedExecutionError {
                probe_identifier: probe.identifier(),
                source: Box::new(e),
            })?;
        if keys.len() != 1 {
            return Err(InquestError::EmptySource);
        }

        Ok(rustls::PrivateKey(keys[0].clone()))

        // let mut data = Vec::new();
        // keyfile.read_to_end(&mut data)?;
        // Ok(rustls::PrivateKey(data))
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
            None,
        );

        assert_eq!("localhost", &probe.host);
        assert_eq!(5432, probe.port);
        assert_eq!("postgres", &probe.database);
    }
}
