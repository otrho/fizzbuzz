mod common;

use common::test_str;

#[test]
fn test_for() {
    test_str(SMALL_LOOP_CODE, "  0\n  1\n  2\n  3\n  4\n");
    test_str(HIGH_LOOP_CODE, "666\n667\n668\n");
    test_str(ONE_ITER_CODE, "  2\n");
    test_str(NO_ITER_CODE, "");
}

const SMALL_LOOP_CODE: &str = r#"
for (thing; 0, 4) {
    print(thing);
}
"#;

const HIGH_LOOP_CODE: &str = r#"
for (z; 666, 668) {
    print(z);
}
"#;

const ONE_ITER_CODE: &str = r#"
for (singularity; 2, 2) {
    print(singularity);
}
"#;

const NO_ITER_CODE: &str = r#"
for (nil; 3, 2) {
    print(nil);
}
"#;
