use reqwest::blocking::*;
use reqwest::{Certificate, Identity, StatusCode};
use std::fs::File;
use url::Url;

use crate::error::InquestError::{AssertionMatchingError, FailedExecutionError};
use crate::{Certificates, Result};
use crate::{GlobalOptions, Http, Probe, ProbeReport};
use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::vec;
use std::{fs, io};

const PROBE_NAME: &str = "HTTP";

impl Http {
    pub fn new(
        url: Url,
        status: Option<u16>,
        name: Option<String>,
        options: &'static GlobalOptions,
        certs: Option<Certificates>,
    ) -> Http {
        Http {
            options,
            url,
            status: status.unwrap_or(200),
            name,
            certs,
        }
    }
}

impl Probe for Http {
    fn execute<'a>(&self) -> Result<ProbeReport> {
        let client = build_client(self)?;
        validate_result(client.get(self.url.as_str()).send(), self)
    }
    fn identifier(&self) -> String {
        format!("{} - {}", PROBE_NAME, self.url)
    }
}

fn build_client(config: &Http) -> Result<Client> {
    let mut cb = Client::builder();
    cb = cb.timeout(config.options.timeout);
    if let Some(cert_option) = &config.certs {
        // important, otherwise certs not working
        cb = cb.use_rustls_tls();
        // Add CA-Cert if available
        if let Some(cacert) = &cert_option.ca_cert {
            let buf = fs::read(cacert)?;
            cb = cb.add_root_certificate(Certificate::from_pem(&buf)?);
        }
        // Add Client-Cert if available
        if let Some(client_pem) = &cert_option.client_pem {
            let mut buf = Vec::new();
            File::open(client_pem)?.read_to_end(&mut buf)?;
            cb = cb.identity(Identity::from_pem(&buf)?);
        }
    }
    Ok(cb.build()?)
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
        let probe = Http::new(
            Url::parse("http://www.foo.bar").unwrap(),
            None,
            None,
            &GO,
            None,
        );

        assert_eq!(200, probe.status);
    }
}
