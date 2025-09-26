#[test_case::test_case("missing_impl.rs"; "missing implementation in Dev")]
fn test_failures(path: &'static str) {
    let t = trybuild::TestCases::new();
    t.compile_fail(format!("tests/failures/{path}"));
}
