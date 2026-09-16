# spacetime init projects/ auto-detect smoke test

## With projects/ present (auto-detect creates project at projects/<name>/)

```
$ mkdir -p $tmpdir/projects && cd $tmpdir && spacetime init demo-with
▶ Creating Spacetime project: /tmp/tmp.HsR2fRH9u0/projects/demo-with
  ✓ index.html
  ✓ index.st
  ✓ styles.css

✓ Project created successfully!

Next steps:
  cd projects/demo-with
  spacetime serve .
  # Or from the Spacetime repo root:
  cargo run -- serve projects/demo-with
```

## Without projects/ (preserves prior behavior, creates ./<name>/)

```
$ cd $tmpdir && spacetime init demo-without
▶ Creating Spacetime project: /tmp/tmp.wEmqAaaEpO/demo-without
  ✓ index.html
  ✓ index.st
  ✓ styles.css

✓ Project created successfully!

Next steps:
  cd demo-without
  spacetime serve .
  # Or from the Spacetime repo root:
  cargo run -- serve demo-without
```

## Helper unit tests

```
$ cargo test --bin spacetime init_target_tests
running 5 tests
test init_target_tests::resolve_init_target_no_projects_dir ... ok
test init_target_tests::resolve_init_target_projects_is_file ... ok
test init_target_tests::resolve_init_target_returns_pathbuf ... ok
test init_target_tests::resolve_init_target_with_projects_dir ... ok
test init_target_tests::resolve_init_target_with_projects_symlink_to_dir ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
