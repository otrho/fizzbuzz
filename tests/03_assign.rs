mod common;

use common::test_str;

#[test]
fn test_assign() {
    test_str(&wrap_in_ifelse(&["a = 42;"], "a == 42"), "True!\n");
    test_str(&wrap_in_ifelse(&["a = 42;"], "a == 69"), "False!\n");
    test_str(&wrap_in_ifelse(&["a = 42;", "b = 69;"], "a == 42 && b == 69"), "True!\n");
    test_str(&wrap_in_ifelse(&["a = 42;", "b = 69;"], "a == 42 && b == 42"), "False!\n");
}

// TODO: assigning strings?!

fn wrap_in_ifelse(assigns: &[&str], test_expr: &str) -> String {
    let mut out_str = String::new();
    out_str.push_str("if (1) {");
    for assign in assigns {
        out_str.push_str(assign);
    }
    out_str.push_str("if (");
    out_str.push_str(test_expr);
    out_str.push_str(r#") { print("True!"); } else { print("False!"); } } else { print("if/else failure!"); }"#);
    out_str
}
