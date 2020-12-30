use hocon::Hocon;
use crate::core::{Config, Oracle};
use crate::core::error::InquestError;
use secrecy::SecretString;
use crate::input::parser::{parse_sql, GO};
use crate::core::Result;

pub(crate) fn parse_oracle(hocon: &Hocon) -> Result<Vec<Config>> {
    if let Hocon::Array(oracles) = &hocon {
        Ok(oracles.into_iter().map(|x|parse(x)).flatten().collect())
    } else {
        Err(InquestError::ConfigurationError)
    }
}

fn parse(hocon: &Hocon) -> Result<Config> {
    // TODO collect all errors at once if possible
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
    let sid = hocon["sid"]
        .as_string()
        .ok_or(InquestError::ConfigurationError)?;
    let sql = parse_sql(&hocon)?;

    Ok(Oracle::new(
        host,
        port,
        sid,
        user,
        password,
        sql,
        &GO,
    ).into())
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret};
    use crate::core::{Oracle, Config};
    use crate::input::parser::tests::match_content;

    #[test]
    fn parse_oracle() {
        let content = r#"
            probe-specification {
                my-service {
                    oracle = [{
                        host = "localhost"
                        host = ${?ORACLE_HOST}
                        port = 1521 # can be omitted when 1521
                        sid = "XE"
                        user = "SYSTEM"
                        password = "hX8AgBVOd/GvecheybpEPA==" # 'changeit'
                        sql {
                            query = "select * from V$SESSION_CONNECT_INFO"
                        }
                    }]
                }
            }"#;
        match_content(content, |config| match config {
            Config::Oracle(Oracle { host, port, sid, user, password, sql, .. }) => {
                assert_eq!("localhost", host);
                assert_eq!(1521, *port);
                assert_eq!("XE", sid);
                assert_eq!("SYSTEM", user);
                assert_eq!("hX8AgBVOd/GvecheybpEPA==", password.expose_secret());
                assert_eq!(true, sql.as_ref().unwrap().query.len() > 0);
            }
            _ => panic!("did not match Oracle probe")
        });
    }

}