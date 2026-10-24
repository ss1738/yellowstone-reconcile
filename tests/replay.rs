use std::process::Command;

#[test]
fn agreement_fixture_emits_once_despite_connection_restart() {
    let output = Command::new(env!("CARGO_BIN_EXE_yellowstone-replay"))
        .arg("fixtures/agreement.jsonl")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines
            .iter()
            .filter(|v| v["outcome"]["kind"] == "emitted")
            .count(),
        1
    );
    assert_eq!(lines.last().unwrap()["stats"]["emitted"], 1);
    assert_eq!(lines.last().unwrap()["stats"]["duplicates"], 1);
}

#[test]
fn conflicting_fixture_halts_without_emitting_a_block() {
    let output = Command::new(env!("CARGO_BIN_EXE_yellowstone-replay"))
        .arg("fixtures/conflict.jsonl")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"kind\":\"conflict\""));
    assert!(!text.contains("\"kind\":\"emitted\""));
}

#[test]
fn malformed_replay_and_bad_argument_are_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_yellowstone-replay"))
        .arg("fixtures/malformed.jsonl")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("line 1"));
    let output = Command::new(env!("CARGO_BIN_EXE_yellowstone-replay"))
        .args(["fixtures/agreement.jsonl", "0"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
