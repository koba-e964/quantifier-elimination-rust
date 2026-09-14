use std::process::Command;

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
        "(-x2^2 + 2*x3 + x4 = 0) && (x2^2 - 4*x3 >= 0)\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("cells constructed: 0"));
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
