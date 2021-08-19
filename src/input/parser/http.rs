use hocon::Hocon;
use url::Url;

use crate::error::InquestError;
use crate::input::parser::GO;
use crate::Result;
use crate::{Config, Http};

pub(crate) fn parse_http(hocon: &Hocon) -> Result<Vec<Config>> {
    if let Hocon::Array(http_specs) = &hocon {
        Ok(http_specs
            .into_iter()
            .flat_map(|hocon| parse_get(hocon))
            .map(|parsed| parsed.into())
            .collect())
    } else {
        Err(InquestError::ConfigurationError)
    }
}

fn parse_get(hocon: &Hocon) -> Result<Http> {
    let url = hocon["url"]
        .as_string()
        .ok_or(InquestError::ConfigurationError)?;
    let status = hocon["status"].as_i64().map(|number| number as u16);
    let name = hocon["name"].as_string();
    Ok(Http::new(Url::parse(url.as_str())?, status, name, &GO))
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use crate::input::parser::tests::match_content;
    use crate::{Config, Http};

    #[test]
    fn parse_http() {
        let content = r#"
            probe-specification {

                my-service {
                    http = [
                        {
                            name = "Testing GET against HTTPBin"
                            url = "https://httpbin.org/get"
                            status = 200
                        }
                    ]
                }
            }"#;
        match_content(content, |config| match config {
            Config::Http(Http {
                url, status, name, ..
            }) => {
                assert_eq!("https://httpbin.org/get", url.to_string());
                assert_eq!(200, *status);
                assert_eq!(
                    "Testing GET against HTTPBin",
                    name.deref().as_ref().unwrap()
                );
            }
            _ => panic!("did not match HTTP probe"),
        });
    }
}
