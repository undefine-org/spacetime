//! PLAN-076 W3 — `spacetime migrate` end-to-end against a fixture project:
//! rewrite old syntax on disk, bump @version, idempotent rerun, dry-run
//! purity, and --wave ordering. Drives the real binary
//! (CARGO_BIN_EXE_spacetime) from the repo root (stdlib discovery is
//! CWD-relative, like production CLI use).

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_spacetime"))
}

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("mig-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.st"),
            "// fixture root\n$user object: { name: \"Ada\" };\n.card { @bind(text: $user.name) }\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("details.st"),
            "$flag bool: true;\n.badge { @bind(visible: $flag) }\n",
        )
        .unwrap();
        Fixture(dir)
    }
    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.0.join(name)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn migrate_rewrites_files_bumps_version_and_is_idempotent() {
    let fx = Fixture::new();

    // Dry run: prints hunks, changes NOTHING on disk (incl. no @version).
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("reactive-surface"), "stdout: {stdout}");
    assert!(
        stdout.contains("@version: (none) -> 2026-06-09"),
        "stdout: {stdout}"
    );
    assert!(
        fx.read("index.st").contains("@bind(text:"),
        "dry run must not write"
    );

    // Apply.
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let index = fx.read("index.st");
    assert!(index.contains("text <- $user.name;"), "index: {index}");
    assert!(index.contains("@version 2026-06-09;"), "index: {index}");
    let details = fx.read("details.st");
    assert!(details.contains(".shown: $flag;"), "details: {details}");

    // Rerun: structurally idempotent (the version fact makes the wave inert).
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("current"), "stdout: {stdout}");
    assert_eq!(fx.read("index.st"), index, "second run must be a no-op");
}

#[test]
fn migrate_wave_refuses_to_skip_older_pending_waves() {
    let fx = Fixture::new();

    // Requesting a newer wave while 2026-06-09 is pending is REFUSED —
    // waves apply oldest-first.
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--wave", "2027-01-01"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be applied yet") && stderr.contains("2026-06-09"),
        "stderr: {stderr}"
    );

    // A malformed wave date is a usage error.
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--wave", "soon"])
        .output()
        .unwrap();
    assert!(!out.status.success());

    // Applying the (only) pending wave explicitly works; after it, a newer
    // (empty) wave is trivially fine.
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--wave", "2026-06-09"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(fx.read("index.st").contains("text <- $user.name;"));
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--wave", "2027-01-01"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("nothing to do"), "stdout: {stdout}");
}

#[test]
fn explain_shows_capsule_rules_hints_and_project_sites() {
    let fx = Fixture::new();
    let out = bin()
        .args([
            "migrate",
            fx.0.to_str().unwrap(),
            "--explain",
            "reactive-surface",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The capsule contract: wave, retired defs, rules (old → new), hints.
    // (ANSI-bolded header — assert the parts, not the escape-joined whole.)
    assert!(
        stdout.contains("migration `reactive-surface`"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("wave 2026-06-09"), "stdout: {stdout}");
    assert!(stdout.contains("%macro bind — @bind"), "stdout: {stdout}");
    assert!(stdout.contains("bind-text"), "stdout: {stdout}");
    assert!(
        stdout.contains("- @bind(text: $x:expr)"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("+ text <- `$x`;"), "stdout: {stdout}");
    assert!(stdout.contains("@show:"), "stdout: {stdout}");
    // The fixture has 2 sites (index @bind(text:), details @bind(visible:)).
    assert!(stdout.contains("2 site(s) pending"), "stdout: {stdout}");

    // Unknown id: exit 2 + names the known migrations.
    let out = bin()
        .args(["migrate", fx.0.to_str().unwrap(), "--explain", "nope"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("reactive-surface"), "stderr: {stderr}");
}

#[test]
fn scaffold_writes_a_dated_capsule_and_refuses_overwrite() {
    // Run in a temp CWD so the scaffold lands in a throwaway stdlib tree —
    // the real entries dir must never carry test artifacts.
    let dir = std::env::temp_dir().join(format!("mig-scaffold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = bin()
        .args(["migrate", ".", "--scaffold", "widget-retire"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let entries = dir.join("stdlib/migrations/entries");
    let files: Vec<_> = std::fs::read_dir(&entries).unwrap().collect();
    assert_eq!(files.len(), 1);
    let name = files[0]
        .as_ref()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .to_string();
    assert!(name.ends_with("-widget-retire.st"), "name: {name}");
    // Wave date is a valid ISO date prefix (filename drives wave ordering).
    let date = &name[..10];
    assert!(spacetime::migrate::is_iso_wave_date(date), "date: {date}");
    let body = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert!(body.contains("%migration widget-retire"), "body: {body}");
    assert!(body.contains("%rewrite old-to-new"), "body: {body}");

    // Second run refuses to overwrite.
    let out = bin()
        .args(["migrate", ".", "--scaffold", "widget-retire"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_at_version_reports_would_break_as_e0911() {
    let fx = Fixture::new();
    // At the wave date: both files' uses are version errors.
    let out = bin()
        .args([
            "check",
            fx.0.to_str().unwrap(),
            "--at-version",
            "2026-06-09",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("E0911"), "stdout: {stdout}");
    assert!(stdout.contains("still uses @bind"), "stdout: {stdout}");

    // BEFORE the wave: nothing inert — the files pass (pending notice only).
    let out = bin()
        .args([
            "check",
            fx.0.to_str().unwrap(),
            "--at-version",
            "2026-01-01",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    // Bad date shape: exit 2.
    let out = bin()
        .args(["check", fx.0.to_str().unwrap(), "--at-version", "banana"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
