use std::process::Command;
use std::process::{Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn run_cli(formula: &str) -> String {
    run_cli_args([formula])
}

fn run_cli_args<const N: usize>(args: [&str; N]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args(args)
        .output()
        .unwrap();
    format!(
        "status: {}\nstdout:\n{}stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

fn run_cli_args_with_timeout<const N: usize>(args: [&str; N]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if started.elapsed() >= Duration::from_secs(2) {
            child.kill().unwrap();
            panic!("snapshot CLI command exceeded the two-second bound");
        }
        sleep(Duration::from_millis(10));
    }
}

#[test]
fn snapshots_closed_formula_output() {
    insta::assert_snapshot!("closed_formula_output", run_cli("exists x0. x0^2 + 1 = 0"));
}

#[test]
fn snapshots_universal_formula_output() {
    insta::assert_snapshot!(
        "universal_formula_output",
        run_cli("forall x0. x0^2 + 1 > 0")
    );
}

#[test]
fn snapshots_quantifier_free_output() {
    insta::assert_snapshot!("quantifier_free_output", run_cli("1 = 1 && x > 0"));
}

#[test]
fn snapshots_universal_quadratic_strict_inequality_output() {
    insta::assert_snapshot!(
        "universal_quadratic_strict_inequality_output",
        run_cli("forall x. x^2 + y > 0")
    );
}

#[test]
fn snapshots_irrational_section_output() {
    insta::assert_snapshot!(
        "irrational_section_output",
        run_cli("exists x1. (x0^2 - 2 = 0) && (x1 - x0 = 0)")
    );
}

#[test]
fn snapshots_multivariate_output() {
    insta::assert_snapshot!(
        "multivariate_output",
        run_cli("exists x1. x1^2 + x0 + x2 = 0")
    );
}

#[test]
fn snapshots_named_variable_output() {
    insta::assert_snapshot!(
        "named_variable_output",
        run_cli_args([
            "--stats",
            "--variable-order=st,y",
            "exists x. x^2 + st + y = 0"
        ])
    );
}

#[test]
fn snapshots_unbounded_quadratic_inequality_output() {
    insta::assert_snapshot!(
        "unbounded_quadratic_inequality_output",
        run_cli_args(["--variable-order=y", "exists x. x^2 + y > -1"])
    );
}

#[test]
fn snapshots_nested_quantifier_output() {
    insta::assert_snapshot!(
        "nested_quantifier_output",
        run_cli("forall x2. exists x1. x1^2 + x0*x2 = 0")
    );
}

#[test]
fn snapshots_vieta_quadratic_root_condition() {
    insta::assert_snapshot!(
        "vieta_quadratic_root_condition",
        run_cli("exists x0. exists x1. x2=x0+x1&&x3=x0*x1")
    );
}

#[test]
fn snapshots_parse_error_output() {
    insta::assert_snapshot!("parse_error_output", run_cli("x0 ="));
}

#[test]
fn snapshots_stats_output() {
    insta::assert_snapshot!(
        "stats_output",
        run_cli_args(["--stats", "exists x1. x1^2 + x0 + x2 = 0"])
    );
}

#[test]
fn snapshots_vieta_special_handling_output() {
    insta::assert_snapshot!(
        "vieta_special_handling_output",
        run_cli_args([
            "--stats",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2",
        ])
    );
}

#[test]
fn snapshots_cubic_symmetric_output() {
    insta::assert_snapshot!(
        "cubic_symmetric_output",
        run_cli("exists x. exists y. x^3 + y^3 = 3*x*y && k = x+y")
    );
}

#[test]
fn snapshots_bounded_cubic_regression_output() {
    let output = run_cli_args_with_timeout([
        "--max-cells=100",
        "exists x. exists y. x^3+y^3=3*x*y&&k=x+y",
    ]);
    assert!(output.status.success());
    insta::assert_snapshot!(
        "bounded_cubic_regression_output",
        format!(
            "status: {}\nstdout:\n{}stderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    );
}

#[test]
fn snapshots_implicit_product_output() {
    insta::assert_snapshot!(
        "implicit_product_output",
        run_cli("exists x. exists y. x+y=k && x^2+x*y+y^2=1")
    );
}
