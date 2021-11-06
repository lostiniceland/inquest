#[macro_use]
extern crate assert_matches;

use std::path::Path;
use std::sync::Once;

use libinquest;
use libinquest::error::InquestError;
use libinquest::run_from_config;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        stderrlog::new().verbosity(2).quiet(false).init().unwrap();
    });
}

#[test]
fn run_http_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-http.conf"));
    assert!(result.is_ok());
    assert_matches!(result.unwrap().0.as_slice(), [report1, report2] => {
        assert_eq!(report1.probe_identifier.as_str(), "HTTP - https://httpbin.org/get");
        assert_eq!(report2.probe_identifier.as_str(), "HTTP - https://httpbin.org/status/201");
    });
}

#[test]
fn run_http_probe_assertion_error() {
    setup();
    let result = run_from_config(Path::new("tests/integration-http-fail.conf"));
    assert!(result.is_ok());
    assert_matches!(
        result.unwrap().1.as_slice(),
        [InquestError::AssertionMatchingError(_, _)]
    );
}

#[test]
fn run_postgres_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-postgres.conf"));
    assert!(result.is_ok());
    assert_matches!(result.unwrap().0.as_slice(), [report] => {
        assert_eq!(report.probe_identifier.as_str(), "Postgres - localhost:5432/test/admin");
    });
}

#[test]
fn run_oracle_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-oracle.conf"));
    assert!(result.is_ok());
    assert_matches!(result.unwrap().0.as_slice(), [report] => {
        assert_eq!(report.probe_identifier.as_str(), "Oracle - localhost:1521/XEPDB1/test");
    });
}

#[test]
fn run_mssql_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-mssql.conf"));
    assert!(result.is_ok());
    assert_matches!(result.unwrap().0.as_slice(), [report] => {
        assert_eq!(report.probe_identifier.as_str(), "MSSql - localhost:1433/SA");
    });
}
