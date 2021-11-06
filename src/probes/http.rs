use reqwest::blocking::*;
use reqwest::StatusCode;
use url::Url;

use crate::error::InquestError::{AssertionMatchingError, FailedExecutionError};
use crate::Result;
use crate::{GlobalOptions, Http, Probe, ProbeReport};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::vec;

const PROBE_NAME: &'static str = "HTTP";

impl Http {
    pub fn new(
        url: Url,
        status: Option<u16>,
        name: Option<String>,
        options: &'static GlobalOptions,
    ) -> Http {
        Http {
            options,
            url,
            status: status.unwrap_or(200),
            name,
        }
    }
}

impl Probe for Http {
    fn execute<'a>(&self) -> Result<ProbeReport> {
        let client = Client::builder().timeout(self.options.timeout).build()?;
        validate_result(client.get(self.url.as_str()).send(), self)
    }

    fn identifier(&self) -> String {
        format!("{} - {}", PROBE_NAME, self.url.to_string())
    }
}

fn validate_result(call_result: reqwest::Result<Response>, config: &Http) -> Result<ProbeReport> {
    match call_result {
        Ok(response) => {
            let mut report = ProbeReport {
                probe_identifier: config.identifier(),
                data: Default::default(),
            };

            response.headers().iter().for_each(|header| {
                report.data.push((
                    header.0.to_string(),
                    String::from_utf8(header.1.as_ref().to_vec()).unwrap(),
                ));
            });

            report.data.sort();

            if response.status() != StatusCode::from_u16(config.status).unwrap() {
                let desc = format!(
                    "Expected '{}' but was '{}'",
                    config.status,
                    response.status()
                );
                Err(AssertionMatchingError(desc, report.clone()))
            } else {
                Ok(report)
            }
        }
        Err(source) => Err(FailedExecutionError {
            probe_identifier: config.identifier(),
            source: Box::new(source),
        }),
    }
}

impl ToSocketAddrs for Http {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        format!(
            "{}:{}",
            self.url.host().unwrap(),
            self.url.port_or_known_default().unwrap_or(80)
        )
        .to_socket_addrs()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Http, GO};
    use url::Url;

    #[test]
    fn probe_uses_documented_defaults() {
        let probe = Http::new(Url::parse("http://www.foo.bar").unwrap(), None, None, &GO);

        assert_eq!(200, probe.status);
    }
}
