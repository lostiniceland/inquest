use hocon::Hocon;
use secrecy::SecretString;

use crate::error::InquestError;
use crate::input::parser::{parse_sql, GO};
use crate::Result;
use crate::{Config, MSSql};

pub(crate) fn parse_mssql(hocon: &Hocon) -> Result<Vec<Config>> {
    if let Hocon::Array(mssqls) = &hocon {
        Ok(mssqls.into_iter().map(|x| parse(x)).flatten().collect())
    } else {
        Err(InquestError::ConfigurationError)
    }
}

// FIXME improve error handling on missing configs
fn parse(hocon: &Hocon) -> Result<Config> {
    let host = hocon["host"].as_string();
    let port = hocon["port"].as_i64().map(|port| port as u16);
    let user = hocon["user"]
        .as_string()
        .ok_or(InquestError::ConfigurationError)?;
    let password = SecretString::new(
        hocon["password"]
            .as_string()
            .ok_or(InquestError::ConfigurationError)?,
    );
    let sql = parse_sql(&hocon)?;
    Ok(MSSql::new(host, port, user, password, sql, &GO).into())
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use crate::input::parser::tests::match_content;
    use crate::{Config, MSSql};

    #[test]
    fn parse_mssql() {
        let content = r#"
            probe-specification {
                my-service {
                    mssql = [{
                        host = "localhost"
                        host = ${?MSSQL_HOST} # can be omitted when 'localhost'
                        port = 1433 # can be omitted when 1433
                        user = "SA"
                        password = "rLg3oWW5DLVUH+1rHu502g==" # 'changeit_C8'
                        sql {
                            query = "SELECT * FROM sys.databases;"
                        }
                    }]
                }
            }"#;
        match_content(content, |config| match config {
            Config::MSSql(MSSql {
                host,
                port,
                user,
                password,
                sql,
                ..
            }) => {
                assert_eq!("localhost", host);
                assert_eq!(1433, *port);
                assert_eq!("SA", user);
                assert_eq!("rLg3oWW5DLVUH+1rHu502g==", password.expose_secret());
                assert_eq!(true, sql.as_ref().unwrap().query.len() > 0);
            }
            _ => panic!("did not match MSSQL probe"),
        });
    }
}
