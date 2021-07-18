mod parser;

use std::io::Read;

fn main() -> Result<(), std::io::Error> {
    let matches = clap::App::new("fbl")
        .version(std::env!("CARGO_PKG_VERSION"))
        .author(std::env!("CARGO_PKG_AUTHORS"))
        .about("FizzBuzz Language :)")
        .args(&[clap::Arg::with_name("FILE").required(true)])
        .get_matches();

    let src_path = matches.value_of("FILE").unwrap_or_default();
    let mut src_file = std::fs::File::open(src_path)?; //.map_err(|err| format!("{}", err))?;
    let mut input_string = String::new();
    src_file.read_to_string(&mut input_string)?;

    let ast = parser::parse_string(&input_string)?;

    println!("{:?}", ast);

    println!("Done.");

    Ok(())
}
