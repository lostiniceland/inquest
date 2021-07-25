use std::path::Path;
use std::sync::Once;

use libinquest;
use libinquest::core::run_from_config;

static INIT: Once = Once::new();


fn setup() {
    INIT.call_once(|| {
        stderrlog::new().verbosity(2).quiet(false).init().unwrap();
    });
}


#[test]
fn run_http_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-http.conf")).unwrap();
    println!("{:#?}", result);
    assert_eq!(result.0.len(), 2);
}

#[test]
fn run_http_probe_assertion_error() {
    setup();
    assert_eq!(run_from_config(Path::new("tests/integration-http-fail.conf")).unwrap().1.len(), 1);
}

#[test]
fn run_postgres_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-postgres.conf")).unwrap();
    println!("{:#?}", result);
    assert_eq!(result.0.len(), 1);
}

#[test]
fn run_oracle_probe() {
    match option_env!("TEST_ENV") {
        Some("github") => println!("skipping Oracle test on Github-Actions!"),
        _ => {
            setup();
            let result = run_from_config(Path::new("tests/integration-oracle.conf")).unwrap();
            println!("{:#?}", result);
            assert_eq!(1, result.0.len());
        }
    }
}

#[test]
fn run_mssql_probe() {
    setup();
    let result = run_from_config(Path::new("tests/integration-mssql.conf")).unwrap();
    println!("{:#?}", result);
    assert_eq!(result.0.len(), 1);
}


