mod common;

use common::test_str;

#[test]
fn test_for() {
    test_str(FIZZ_BUZZ_CODE, FIZZ_BUZZ_OUTPUT);
}

const FIZZ_BUZZ_CODE: &str = r#"
for (i; 1, 20) {
  is_mult3 = i % 3 == 0;
  is_mult5 = i % 5 == 0;
  if (is_mult3 && is_mult5) {
    print("FizzBuzz");
  } else {
    if (is_mult3) {
      print("Fizz");
    } else {
      if (is_mult5) {
        print("Buzz");
      } else {
        print(i);
      }
    }
  }
}
"#;

const FIZZ_BUZZ_OUTPUT: &str = r#"  1
  2
Fizz
  4
Buzz
Fizz
  7
  8
Fizz
Buzz
 11
Fizz
 13
 14
FizzBuzz
 16
 17
Fizz
 19
Buzz
"#;
