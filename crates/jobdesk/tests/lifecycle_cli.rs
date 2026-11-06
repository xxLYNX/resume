//! CLI lifecycle transitions on disposable workspaces (same store path as HTTP PATCH).

use std::path::PathBuf;
use std::process::Command;

use domain::Composition;
use store::{CreateApplicationRequest, Store, UpdateApplicationRequest};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn jobdesk(args: &[&str], workspace: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_jobdesk"))
        .arg("--root")
        .arg(repo_root())
        .arg("--workspace")
        .arg(workspace)
        .args(args)
        .output()
        .expect("run jobdesk")
}

#[test]
fn cli_update_status_only_after_submit() {
    let ws = tempfile::tempdir().unwrap();
    let ws_path = ws.path().to_path_buf();
    Store::open(&ws_path.join("jobdesk.db")).unwrap();

    let add = jobdesk(&["application", "add", "Co", "Eng"], &ws_path);
    assert!(add.status.success());
    let app_id = String::from_utf8(add.stdout).unwrap().trim().to_string();

    let store = Store::open(&ws_path.join("jobdesk.db")).unwrap();
    store.connection().execute(
        "INSERT INTO submissions (id, application_id, manifest_path, content_hash, is_canonical, submitted_at)
         VALUES ('s1', ?1, 'artifacts/manifests/s1.json', 'sha256:abc', 1, '2020-01-01T00:00:00Z')",
        [&app_id],
    ).unwrap();
    store
        .connection()
        .execute(
            "UPDATE applications SET phase='submitted', status='submitted' WHERE id=?1",
            [&app_id],
        )
        .unwrap();

    let upd = jobdesk(
        &["application", "update", &app_id, "--status", "interview"],
        &ws_path,
    );
    assert!(upd.status.success(), "{:?}", upd.stderr);

    let app = store.get_application(&app_id).unwrap().unwrap();
    assert_eq!(app.phase, "submitted");
    assert_eq!(app.status, "interview");
}

#[test]
fn cli_set_status_interview_without_submit_fails() {
    let ws = tempfile::tempdir().unwrap();
    let ws_path = ws.path().to_path_buf();
    Store::open(&ws_path.join("jobdesk.db")).unwrap();

    let add = jobdesk(&["application", "add", "Co", "Eng"], &ws_path);
    let app_id = String::from_utf8(add.stdout).unwrap().trim().to_string();

    let set = jobdesk(
        &["application", "set-status", &app_id, "interview"],
        &ws_path,
    );
    assert!(!set.status.success());
}

#[test]
fn store_http_equivalent_contradictory_phase_workflow_rejected() {
    let ws = tempfile::tempdir().unwrap();
    let db = ws.path().join("jobdesk.db");
    let store = Store::open(&db).unwrap();
    let app = store
        .create_application(
            CreateApplicationRequest {
                company: "Co".into(),
                title: "T".into(),
                posting_url: None,
                posting_text: None,
                source: None,
                status: None,
                tags: vec![],
                clone_from_application_id: None,
                clone_from_preset_id: None,
                employment_type: None,
                work_mode: None,
                location: None,
                compensation: None,
            },
            Composition { sections: vec![] },
        )
        .unwrap();

    let err = store
        .update_application(
            &app.id,
            UpdateApplicationRequest {
                phase: Some("draft".into()),
                status: Some("interview".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(err.to_string().contains("invalid for phase `draft`"));
}
