use reqwest::blocking::*;
use reqwest::StatusCode;
use url::Url;

use crate::error::InquestError::{AssertionError, FailedExecutionError};
use crate::Result;
use crate::{GlobalOptions, Http, Probe, ProbeReport};

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
}

fn validate_result(call_result: reqwest::Result<Response>, config: &Http) -> Result<ProbeReport> {
    match call_result {
        Ok(response) => {
            let mut report = ProbeReport {
                probe_name: PROBE_NAME,
                probe_identifier: config.url.to_string(),
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
                Err(AssertionError(report.clone()))
            } else {
                Ok(report)
            }
        }
        Err(source) => Err(FailedExecutionError {
            source: Box::new(source),
        }),
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
