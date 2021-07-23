mod common;

use common::test_str;

#[test]
fn test_binops_eq() {
    test_str(&wrap_in_ifelse("1 == 1"), "True!\n");
    test_str(&wrap_in_ifelse("1 == 0"), "False!\n");
    test_str(&wrap_in_ifelse("0 == 0"), "True!\n");
    test_str(&wrap_in_ifelse("1234 == 1234"), "True!\n");
}

#[test]
fn test_binops_and() {
    test_str(&wrap_in_ifelse("1 && 1"), "True!\n");
    test_str(&wrap_in_ifelse("1 && 0"), "False!\n");
    test_str(&wrap_in_ifelse("0 && 0"), "False!\n");
    test_str(&wrap_in_ifelse("1 && 2"), "True!\n");
    test_str(&wrap_in_ifelse("1 && 32"), "True!\n");
    test_str(&wrap_in_ifelse("1234 && 1234"), "True!\n");
    test_str(&wrap_in_ifelse("1234 && 4321"), "True!\n");
}

#[test]
fn test_binops_mod() {
    test_str(&wrap_in_ifelse("10 % 3 == 1"), "True!\n");
    test_str(&wrap_in_ifelse("555 % 3 == 0"), "True!\n");
    test_str(&wrap_in_ifelse("3 % 555 == 3"), "True!\n");
}

#[test]
fn test_binops_combo() {
    test_str(&wrap_in_ifelse("1 == 1 == 1"), "True!\n");
    test_str(&wrap_in_ifelse("1 == 0 == 1"), "False!\n");
    test_str(&wrap_in_ifelse("1 == 0 == 0"), "True!\n");

    test_str(&wrap_in_ifelse("1 && 1 && 1"), "True!\n");
    test_str(&wrap_in_ifelse("1 && 0 && 1"), "False!\n");
    test_str(&wrap_in_ifelse("1 && 1 && 0"), "False!\n");

    test_str(&wrap_in_ifelse("11 % 8 % 3 == 0"), "True!\n");
    test_str(&wrap_in_ifelse("11 % (8 % 3) == 1"), "True!\n");

    test_str(&wrap_in_ifelse("15 % 3 == 0 && 15 % 5 == 0"), "True!\n");
}

fn wrap_in_ifelse(expr: &str) -> String {
    let mut out_str = String::new();
    out_str.push_str("if (");
    out_str.push_str(expr);
    out_str.push_str(r#") { print("True!"); } else { print("False!"); }"#);
    out_str
}
