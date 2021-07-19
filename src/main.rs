use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::io::Read;

mod parser;

// -------------------------------------------------------------------------------------------------

fn main() -> Result<(), std::io::Error> {
    let matches = clap::App::new("fbl")
        .version(std::env!("CARGO_PKG_VERSION"))
        .author(std::env!("CARGO_PKG_AUTHORS"))
        .about("FizzBuzz Language :)")
        .args(&[clap::Arg::with_name("FILE").required(true)])
        .get_matches();

    // Read the source into a string.
    let src_path = matches.value_of("FILE").unwrap_or_default();
    let mut src_file = std::fs::File::open(src_path)?;
    let mut input_string = String::new();
    src_file.read_to_string(&mut input_string)?;

    // Parse into an AST.
    let ast = parser::parse_string(&input_string)?;

    // Create a JIT module.
    let builder = JITBuilder::new(cranelift_module::default_libcall_names());
    let mut module = JITModule::new(builder);

    // We have an implicit main() which takes and returns nothing.
    let mut ctx = module.make_context();
    ctx.func.signature = module.make_signature();
    let fn_main = module
        .declare_function("main", Linkage::Export, &ctx.func.signature)
        .map_err(|err| to_other_err(err))?;
    ctx.func.name = ExternalName::user(0, fn_main.as_u32());

    // Build our main function and an entry block.
    let mut fn_ctx = FunctionBuilderContext::new();
    let mut fn_builder = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);
    let mut block = fn_builder.create_block();
    fn_builder.switch_to_block(block);
    fn_builder.seal_block(block);

    // Translate our AST into IR.
    compile(&mut fn_builder, &mut block, ast);

    // Finalize the main function.
    fn_builder.seal_all_blocks();
    fn_builder.finalize();
    let mut trap_sink = codegen::binemit::NullTrapSink {};
    let mut stack_map_sink = codegen::binemit::NullStackMapSink {};
    module
        .define_function(fn_main, &mut ctx, &mut trap_sink, &mut stack_map_sink)
        .map_err(|err| to_other_err(err))?;

    // Link.
    module.finalize_definitions();

    // Call the compiled binary (by casting it to fn()).
    let code = module.get_finalized_function(fn_main);
    let ptr = unsafe { std::mem::transmute::<_, fn()>(code) };
    ptr();

    Ok(())
}

// -------------------------------------------------------------------------------------------------

fn to_other_err<E>(err: E) -> std::io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

// -------------------------------------------------------------------------------------------------

fn compile(fn_builder: &mut FunctionBuilder, block: &mut Block, ast: parser::AstNode) {

}

// -------------------------------------------------------------------------------------------------

