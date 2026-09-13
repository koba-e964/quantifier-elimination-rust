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
