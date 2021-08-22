use hocon::Hocon;
use log::warn;

use crate::error::InquestError;
use crate::input::parser::http::parse_http;
use crate::input::parser::mssql::parse_mssql;
use crate::input::parser::oracle::parse_oracle;
use crate::input::parser::postgres::parse_postgres;
use crate::Result;
use crate::{Config, ServiceSpecification, SqlTest};

mod http;
mod mssql;
mod oracle;
mod postgres;

pub fn parse(hocon: &Hocon) -> Result<Vec<ServiceSpecification>> {
    let root = &hocon["probe-specification"];
    // let options =

    let result = match root {
        Hocon::Hash(service) => service
            .into_iter()
            .filter(|(_, v)| {
                let http_present = match v["http"] {
                    Hocon::Array(_) => true,
                    _ => false,
                };
                let oracle_present = match v["oracle"] {
                    Hocon::Array(_) => true,
                    _ => false,
                };
                let postgres_present = match v["postgres"] {
                    Hocon::Array(_) => true,
                    _ => false,
                };
                let mssql_present = match v["mssql"] {
                    Hocon::Array(_) => true,
                    _ => false,
                };
                http_present || oracle_present || postgres_present || mssql_present
            })
            .filter_map(|(k, v)| parse_service(k, v).ok())
            .collect::<Vec<ServiceSpecification>>(),
        _ => Default::default(),
    };

    Ok(result)
}

fn parse_service(service: &String, hocon: &Hocon) -> Result<ServiceSpecification> {
    let probe_configs = match &hocon {
        Hocon::Hash(s) => s
            .iter()
            .map(|(k, v)| match k.as_str() {
                "http" => parse_http(v),
                "postgres" => parse_postgres(v),
                "oracle" => parse_oracle(v),
                "mssql" => parse_mssql(v),
                other => {
                    warn!("Unrecognized Probe '{}' in '{}'", other, service);
                    Err(InquestError::ConfigurationError)
                }
            })
            .flat_map(|result| match result {
                Ok(config) => Some(config),
                Err(_) => None,
            })
            .flatten()
            .collect::<Vec<Config>>(),
        _ => Vec::with_capacity(0),
    };
    // FIXME
    Ok(ServiceSpecification {
        service: service.to_string(),
        probe_configs,
    })
}

fn parse_sql(hocon: &Hocon) -> Result<Option<SqlTest>> {
    if let Hocon::BadValue(_) = hocon["sql"] {
        return Ok(None);
    };

    match hocon["sql"]["query"].as_string() {
        Some(query) => Ok(Some(SqlTest { query })),
        None => Err(InquestError::ConfigurationError),
    }
}

#[cfg(test)]
mod tests {
    use crate::input::parser::parse;
    use crate::{Config, ServiceSpecification};

    pub fn setup(content: &str) -> Vec<ServiceSpecification> {
        let root = hocon::HoconLoader::new()
            .no_url_include()
            .load_str(content)
            .unwrap()
            .hocon()
            .unwrap();

        parse(&root).unwrap()
    }

    pub(crate) fn match_content<T>(content: &str, matcher: T)
    where
        T: Fn(&Config),
    {
        let mut matched = false;
        let spec = setup(content);
        for service in &spec {
            for p in &service.probe_configs {
                &matcher(p);
                matched = true;
            }
        }
        if !matched {
            panic!("basic parsing failed")
        }
    }
}
