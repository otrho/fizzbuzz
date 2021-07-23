mod common;

use common::test_str;

#[test]
fn test_if_stmts() {
    test_str(TRUE_BRANCH_CODE, "Success!\n");
    test_str(FALSE_BRANCH_CODE, "Success!\n");
}

const TRUE_BRANCH_CODE: &str = r#"
if (1) {
  print("Success!");
} else {
  print("Failure!");
}
"#;

const FALSE_BRANCH_CODE: &str = r#"
if (0) {
  print("Failure!");
} else {
  print("Success!");
}
"#;
