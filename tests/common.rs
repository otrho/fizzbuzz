pub fn test_str(input: &str, expected: &str) {
    let output = test_bin::get_test_bin("fizzbuzz")
        .args(&["-e", input])
        .output()
        .expect("Failed to run `fizzbuzz` binary.");

    if !output.status.success() {
        println!("{}\n", String::from_utf8_lossy(&output.stderr));
        panic!("Test failed to compile.")
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str != expected {
        println!("TEST       : test_str");
        println!(" IN        : '{}'", input);
        println!(" EXPECTING : '{}'", expected);
        println!(" GOT       : '{}'", output_str);
        panic!("TEST test_str failed.");
    }
}
