# Targeted cargo test results after reorg

## landing_pages tests (now reading tests/fixtures/landing/{1,2,3}/)

```
$ cargo test --test integration_tests landing_page

running 10 tests
test integration::landing_pages::test_landing_page_1_compiles_clean ... ok
test integration::landing_pages::test_landing_page_1_no_duplicate_const_in_cleanup ... ok
test integration::landing_pages::test_landing_page_1_parses ... ok
test integration::landing_pages::test_landing_page_2_compiles_clean ... ok
test integration::landing_pages::test_landing_page_2_no_duplicate_const_in_cleanup ... ok
test integration::landing_pages::test_landing_page_2_parses ... ok
test integration::landing_pages::test_landing_page_3_compiles_clean ... ok
test integration::landing_pages::test_landing_page_3_no_duplicate_const_in_cleanup ... ok
test integration::landing_pages::test_landing_page_3_parses ... ok
test integration::landing_pages::test_landing_pages_produce_animation_timelines ... ok

test result: ok. 10 passed; 0 failed; 0 ignored
```

## test_runner_site tests (now reading tests/fixtures/test-runner/)

```
$ cargo test --test integration_tests test_runner_site

running 4 tests
test integration::test_runner_site::test_runner_animations_compile ... ok
test integration::test_runner_site::test_runner_html_exists ... ok
test integration::test_runner_site::test_runner_state_machines ... ok
test integration::test_runner_site::test_runner_websocket_integration ... ok

test result: ok. 4 passed; 0 failed; 0 ignored
```

## metasystem stdlib_integration test (now reading tests/fixtures/landing/1/)

```
$ cargo test --lib test_index_st_zero_false_unknown

running 1 test
test metasystem::tests::stdlib_integration::test_index_st_zero_false_unknown_directives ... ok

test result: ok. 1 passed; 0 failed
```

## CLI sanity: spacetime check on relocated landing fixture

```
$ cargo run -- check tests/fixtures/landing/1
✓ tests/fixtures/landing/1/index.st
✓ All 1 file(s) passed
```

## Full suite snapshot (post-reorg, with deletions of private-site tests)

- `cargo test --lib` : 1982 passed, 1 failed (`compiler::snapshot_tests::snapshot_data_directive`).
  - The single failure is an insta snapshot mismatch caused by the prior `c51a096e locale fix for json-provided data` commit. `git diff HEAD --stat src/compiler.rs src/snapshots/` shows zero lines changed by this plan; failure is pre-existing.
- `cargo test --test integration_tests` : 122 passed, 1 failed (`integration::responsive_nested_directives::light_with_nested_loop_emits_guarded_js`).
  - Failure traces back to `2b90a1b8 fix(pipeline): allow nested directives inside @breakpoint` (pre-existing); `git diff HEAD --stat tests/integration/responsive_nested_directives.rs src/pipeline/ stdlib/` shows zero lines changed by this plan.
- 6 private-site tests (compile_ikarchitecte + 2 dependents, compile_oraventures + 3 dependents) intentionally removed because the underlying private sites moved to `projects/` (gitignored, not present in CI).
