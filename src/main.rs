use std::collections::HashMap;
use std::io::Read;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, DataId, Linkage, Module};

mod parser;

use parser::{AstNode, AstValue};

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
    //println!("{:?}", ast);
    //return Ok(());

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

    // Build our main function.
    let mut fn_ctx = FunctionBuilderContext::new();
    let mut fn_builder = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);

    // Translate our AST into IR.
    compile(&mut module, &mut fn_builder, ast);

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
    //println!("{}", ctx.func);

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

fn compile(module: &mut JITModule, fn_builder: &mut FunctionBuilder, program: AstNode) {
    // Gather and declare all our immediate strings.
    let mut data_map = HashMap::<Vec<u8>, DataId>::new();
    let mut str_id = 0_usize;
    compile_data(module, &mut data_map, &mut str_id, &program);
    module.finalize_definitions();

    // Create the entry block.
    let block = fn_builder.create_block();
    fn_builder.switch_to_block(block);
    fn_builder.seal_block(block);

    // Declare all variables in this block.
    let mut var_map = HashMap::<String, Variable>::new();
    let mut var_id = 0_usize;
    declare_all_variables(fn_builder, &mut var_map, &mut var_id, &program);

    // Compile the program.
    compile_code(module, fn_builder, &data_map, &var_map, &program);
    fn_builder.finalize();
}

// -------------------------------------------------------------------------------------------------

fn compile_data(
    module: &mut JITModule,
    data_map: &mut HashMap<Vec<u8>, DataId>,
    str_id: &mut usize,
    program: &AstNode,
) {
    match program {
        AstNode::Literal(AstValue::Text(str_val)) => {
            declare_imm_string(module, data_map, str_val, str_id)
        }
        AstNode::Literal(_) => (),
        AstNode::Identifier(_) => (),
        AstNode::Call(_, args) => args
            .iter()
            .for_each(|arg| compile_data(module, data_map, str_id, arg)),
        AstNode::Assign(_, box_rhs) => compile_data(module, data_map, str_id, box_rhs),
        AstNode::If {
            cond_expr,
            true_expr,
            false_expr,
        } => {
            compile_data(module, data_map, str_id, cond_expr);
            for stmt in true_expr {
                compile_data(module, data_map, str_id, stmt);
            }
            for stmt in false_expr {
                compile_data(module, data_map, str_id, stmt);
            }
        }
        AstNode::For { body, .. } => {
            for stmt in body {
                compile_data(module, data_map, str_id, stmt);
            }
        }
    }
}

fn declare_imm_string(
    module: &mut JITModule,
    data_map: &mut HashMap<Vec<u8>, DataId>,
    str_val: &Vec<u8>,
    str_id: &mut usize,
) {
    let mut data_ctx = DataContext::new();
    data_ctx.define(str_val.clone().into_boxed_slice());

    let name = format!("str_{}", str_id);
    *str_id += 1;

    let id = module
        .declare_data(&name, Linkage::Export, false, false)
        .expect("Declaring a global string immediate.");
    module
        .define_data(id, &data_ctx)
        .expect("Defining a global string immediate.");

    data_map.insert(str_val.clone(), id);
}

// -------------------------------------------------------------------------------------------------

fn declare_all_variables(
    fn_builder: &mut FunctionBuilder,
    var_map: &mut HashMap<String, Variable>,
    var_id: &mut usize,
    program: &AstNode,
) {
    match program {
        AstNode::Assign(name, _) => declare_variable(fn_builder, var_map, name, var_id),
        AstNode::If {
            true_expr,
            false_expr,
            ..
        } => {
            for stmt in true_expr {
                declare_all_variables(fn_builder, var_map, var_id, stmt);
            }
            for stmt in false_expr {
                declare_all_variables(fn_builder, var_map, var_id, stmt);
            }
        }
        AstNode::For { body, .. } => {
            for stmt in body {
                declare_all_variables(fn_builder, var_map, var_id, stmt);
            }
        }

        _ => (),
    }
}

fn declare_variable(
    fn_builder: &mut FunctionBuilder,
    var_map: &mut HashMap<String, Variable>,
    name: &str,
    var_id: &mut usize,
) {
    if !var_map.contains_key(name) {
        let var = Variable::new(*var_id);
        *var_id += 1;

        var_map.insert(name.to_string(), var);
        fn_builder.declare_var(var, types::I64);
    }
}

// -------------------------------------------------------------------------------------------------

fn compile_code(
    module: &mut JITModule,
    fn_builder: &mut FunctionBuilder,
    data_map: &HashMap<Vec<u8>, DataId>,
    _var_map: &HashMap<String, Variable>,
    program: &AstNode,
) {
    match program {
        AstNode::Call(name, args) => {
            // At the moment, for this demo, the only function we do call is `print` and it takes a
            // single literal argument.
            assert!(name == "print");
            assert!(args.len() == 1);
            match &args[0] {
                AstNode::Literal(AstValue::Text(s)) => {
                    compile_print_str(module, fn_builder, data_map, s)
                }
                _ => panic!("Only strings are supported so far..."),
            }
        }
        _ => panic!("unhandled node: {:?}", program),
    }
    fn_builder.ins().return_(&[]);
}

fn compile_print_str(
    module: &mut JITModule,
    fn_builder: &mut FunctionBuilder,
    data_map: &HashMap<Vec<u8>, DataId>,
    str_val: &Vec<u8>,
) {
    // int puts(const char* str)
    let mut sig = module.make_signature();
    let ptr_type = module.target_config().pointer_type();
    sig.params
        .push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I32));

    let libc_puts = module
        .declare_function("puts", Linkage::Import, &sig)
        .expect("Failed to declare `puts()`");
    let callee = module.declare_func_in_func(libc_puts, &mut fn_builder.func);

    let data_id = data_map.get(str_val).unwrap();
    let local_id = module.declare_data_in_func(*data_id, &mut fn_builder.func);

    let arg = fn_builder.ins().symbol_value(ptr_type, local_id);
    fn_builder.ins().call(callee, &vec![arg]);
}

// -------------------------------------------------------------------------------------------------
