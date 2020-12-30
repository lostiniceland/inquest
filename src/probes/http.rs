use reqwest::blocking::*;
use reqwest::StatusCode;
use url::Url;

use crate::core::{Config, GlobalOptions, Http, Probe, ProbeReport};
use crate::core::error::InquestError::{AssertionError, FailedExecutionError};
use crate::core::Result;

const PROBE_NAME: &'static str = "HTTP";

impl Http {
    pub fn new(url: Url, status: Option<u16>, name: Option<String>, options: &'static GlobalOptions) -> Http {
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
        let client = Client::builder()
            .timeout(self.options.timeout)
            .build()?;
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
                metrics: Default::default(),
            };

            response.headers().iter().for_each(|header| {
                report.data.push(
                    (header.0.to_string(),
                    String::from_utf8(header.1.as_ref().to_vec()).unwrap()));
            });

            report.data.sort();

            if response.status() != StatusCode::from_u16(config.status).unwrap() {
                Err(AssertionError(report.clone()))
            } else {
                Ok(report)
            }
        }
        Err(s) => Err(FailedExecutionError)
        // config,
        // error_code: s.status().map(|x|x.as_u16()).unwrap_or(500)}
    }
}





