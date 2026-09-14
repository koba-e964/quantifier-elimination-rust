use std::process::Command;

fn run_cli(formula: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg(formula)
        .output()
        .unwrap();
    format!(
        "status: {}\nstdout:\n{}stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
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
fn snapshots_nested_quantifier_output() {
    insta::assert_snapshot!(
        "nested_quantifier_output",
        run_cli("forall x2. exists x1. x1^2 + x0*x2 = 0")
    );
}

#[test]
fn snapshots_parse_error_output() {
    insta::assert_snapshot!("parse_error_output", run_cli("x0 ="));
}
