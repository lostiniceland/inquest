use hocon::Hocon;
use crate::core::{Config, Postgres};
use crate::core::error::InquestError;
use secrecy::SecretString;
use crate::input::parser::{parse_sql, GO};
use crate::core::Result;

pub(crate) fn parse_postgres(hocon: &Hocon) -> Result<Vec<Config>> {
    if let Hocon::Array(oracles) = &hocon {
        Ok(oracles.into_iter().map(|x|parse(x)).flatten().collect())
    } else {
        Err(InquestError::ConfigurationError)
    }
}

fn parse(hocon: &Hocon) -> Result<Config> {
    let host = hocon["host"].as_string();
    let port = hocon["port"].as_i64().map(|port| port as u16);
    let database = hocon["database"].as_string();
    let user = hocon["user"]
        .as_string()
        .ok_or(InquestError::ConfigurationError)?;
    let password = SecretString::new(
        hocon["password"]
            .as_string()
            .ok_or(InquestError::ConfigurationError)?,
    );
    let sql = parse_sql(&hocon)?;
    Ok(Postgres::new(
        host,
        port,
        database,
        user,
        password,
        sql,
        &GO,
    ).into())
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret};
    use crate::core::{Postgres, Config};
    use crate::input::parser::tests::match_content;

    #[test]
    fn parse_postgres() {
        let content = r#"
            probe-specification {
                my-service {
                    postgres = [{
                        host = "localhost"
                        host = ${?POSTGRES_HOST} # can be omitted when 'localhost'
                        port = 5432 # can be omitted when 5432
                        user = "admin"
                        password = "hX8AgBVOd/GvecheybpEPA==" # 'changeit'
                        database = "test"
                        sql {
                            query = "SELECT (blks_hit*100/(blks_hit+blks_read))::numeric as hit_ratio FROM pg_stat_database WHERE datname='test';"
                        }
                    }]
                }
            }"#;
        match_content(content, |config| match config {
            Config::Postgres(Postgres { host, port, database, user, password, sql, .. }) => {
                assert_eq!("localhost", host);
                assert_eq!(5432, *port);
                assert_eq!("test", database);
                assert_eq!("admin", user);
                assert_eq!("hX8AgBVOd/GvecheybpEPA==", password.expose_secret());
                assert_eq!(true, sql.as_ref().unwrap().query.len() > 0);
            }
            _ => panic!("did not match Postgres probe")
        });
    }


}