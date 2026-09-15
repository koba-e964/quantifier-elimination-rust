use std::process::Command;
use std::process::{Child, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn output_with_timeout(mut child: Child, timeout: Duration) -> Option<Output> {
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return Some(child.wait_with_output().unwrap());
        }
        if started.elapsed() >= timeout {
            child.kill().unwrap();
            child.wait().unwrap();
            return None;
        }
        sleep(Duration::from_millis(10));
    }
}

#[test]
fn eliminates_formula_from_an_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg("exists x0. x0^2 + 1 = 0")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "false\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn reads_formula_from_standard_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qe"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"exists x0. x0 = 0")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true\n");
}

#[test]
fn reports_parse_errors_with_a_nonzero_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg("x0 =")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("parse error"));
}

#[test]
fn rejects_nonpositive_cell_budgets() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args(["--max-cells=0", "exists x0. x0 = 0"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--max-cells requires a positive integer")
    );
}

#[test]
fn reports_when_the_cell_budget_is_exceeded() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--max-cells=1",
            "--special-handling=false",
            "--variable-order=x2,x3,x4",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ComplexityLimitExceeded"));
    assert!(stderr.contains("projection_level"));
    assert!(stderr.contains("variable_order"));
}

#[test]
fn disabled_special_handling_uses_the_bounded_cad_baseline() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--max-cells=1",
            "--special-handling=false",
            "--variable-order=x2,x3,x4",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ComplexityLimitExceeded"));
}

#[test]
fn bounds_the_reported_quadratic_baseline_regression() {
    let child = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--max-cells=100",
            "--special-handling=false",
            "--variable-order=x2,x3,x4",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2",
        ])
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = output_with_timeout(child, Duration::from_secs(5))
        .expect("quadratic CAD baseline exceeded the five-second bound");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ComplexityLimitExceeded"));
}

#[test]
fn bounds_the_reported_cubic_regression() {
    let child = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--max-cells=100",
            "exists x. exists y. x^3+y^3=3*x*y&&k=x+y",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = output_with_timeout(child, Duration::from_secs(2))
        .expect("cubic symmetric elimination exceeded the two-second bound");

    assert!(output.status.success());
}

#[test]
fn prints_linear_multivariate_elimination_results() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg("exists x1. x0*x1 + x2*x1 - 1 = 0")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "x0 + x2 != 0\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn prints_named_variables_in_results() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg("exists x. x^2 + st + y = 0")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "st + y <= 0\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn prints_cad_stats_when_requested() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args(["--stats", "exists x1. x1^2 + x0 + x2 = 0"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "x0 + x2 <= 0\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cells constructed: "));
    assert!(stderr.contains("leaf cells: "));
    assert!(stderr.contains("projection polynomials: "));
}

#[test]
fn applies_symmetric_special_handling_before_cad() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--stats",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "(x2^2 - 2*x3 - x4 = 0) && (x2^2 - 4*x3 >= 0)\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("cells constructed: 0"));
}

#[test]
fn loads_special_rules_from_a_config_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--special-rules=config/special-rules.toml",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "x2^2 - 4*x3 >= 0\n"
    );
}

#[test]
fn special_handling_can_be_disabled_for_a_cad_baseline() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--stats",
            "--special-handling=false",
            "exists x0. exists x1. x2=x0+x1&&x3=x0*x1",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "x2^2 - 4*x3 >= 0\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("cells constructed: "));
}

#[test]
fn special_handling_finishes_the_reported_formula_within_threshold() {
    let child = Command::new(env!("CARGO_BIN_EXE_qe"))
        .arg("exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = output_with_timeout(child, Duration::from_secs(2))
        .expect("special handling exceeded the two-second threshold");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "(x2^2 - 2*x3 - x4 = 0) && (x2^2 - 4*x3 >= 0)\n"
    );
}

#[test]
fn reports_an_explicit_variable_order() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--stats",
            "--variable-order=x2,x0",
            "exists x1. x1^2 + x0 + x2 = 0",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lifting order 0: x2 -> x0 -> x1"));
}

#[test]
fn accepts_named_variable_order() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--stats",
            "--variable-order=st,y",
            "exists x. x^2 + st + y = 0",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lifting order 0: st -> y -> x"));
}

#[test]
fn rejects_duplicate_named_variable_order_entries() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args(["--variable-order=st,st", "exists x. x^2 + st + y = 0"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--variable-order contains duplicate variable: st"));
}

#[test]
fn rejects_an_incomplete_variable_order() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--variable-order=x2,x3",
            "exists x1. x1^2 + x0 + x2 + x3 = 0",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--variable-order must list every free variable exactly once"));
}

#[test]
fn rejects_a_bound_variable_in_the_order() {
    let output = Command::new(env!("CARGO_BIN_EXE_qe"))
        .args([
            "--variable-order=x1,x2,x3",
            "exists x1. x1^2 + x0 + x2 + x3 = 0",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--variable-order must contain exactly the free variables"));
}

#[test]
fn preserves_the_result_across_variable_orders() {
    let outputs = ["x0,x2", "x2,x0"].map(|order| {
        let variable_order = format!("--variable-order={order}");
        let output = Command::new(env!("CARGO_BIN_EXE_qe"))
            .arg(variable_order)
            .arg("exists x1. x1^2 + x0 + x2 = 0")
            .output()
            .unwrap();

        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    });

    assert_eq!(outputs[0], outputs[1]);
}
