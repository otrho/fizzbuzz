mod common;

use common::test_str;

#[test]
fn test_print_simple_numbers() {
    test_str(r#"print(0);"#, "  0\n");
    test_str(r#"print(1);"#, "  1\n");
    test_str(r#"print(10);"#, " 10\n");
    test_str(r#"print(100);"#, "100\n");
    test_str(r#"print(555);"#, "555\n");
    test_str(r#"print(999);"#, "999\n");

    // These are wrong, but the expected behaviour for now.
    test_str(r#"print(1000);"#, "  0\n");
    test_str(r#"print(1001);"#, "  1\n");
    test_str(r#"print(10001);"#, "  1\n");
    test_str(r#"print(99999);"#, "999\n");
}

#[test]
fn test_print_strings() {
    test_str(r#"print("Hello World!");"#, "Hello World!\n");
    test_str(r#"print("");"#, "\n");
}
