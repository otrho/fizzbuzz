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
    let program = parser::parse_string(&input_string)?;
    //println!("{:?}", program);
    //return Ok(());

    // Create a JIT module.
    let builder = JITBuilder::new(cranelift_module::default_libcall_names());
    let mut module = JITModule::new(builder);

    // We have an implicit main() which takes and returns nothing.
    let mut ctx = module.make_context();
    ctx.func.signature = module.make_signature();
    let fn_main = module
        .declare_function("main", Linkage::Export, &ctx.func.signature)
        .map_err(to_other_err)?;
    ctx.func.name = ExternalName::user(0, fn_main.as_u32());

    // Build our main function.
    let mut fn_ctx = FunctionBuilderContext::new();
    let mut fn_builder = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);

    // Gather and declare all our immediate strings.
    let mut data_map = HashMap::<Vec<u8>, DataId>::new();
    let mut str_id = 0_usize;
    compile_data(&mut module, &mut data_map, &mut str_id, &program);
    module.finalize_definitions();

    // Create the entry block.
    let block = fn_builder.create_block();
    fn_builder.switch_to_block(block);
    fn_builder.seal_block(block);

    // Declare all variables in this block.
    let mut var_map = HashMap::<String, Variable>::new();
    let mut var_id = 0_usize;
    declare_all_variables(&mut fn_builder, &mut var_map, &mut var_id, &program);

    // Compile the program.
    let mut compiler = Compiler {
        module: &mut module,
        fn_builder,
        data_map,
        var_map,
    };
    compiler.compile_code(&program);

    // Finalize the main function.
    compiler.fn_builder.ins().return_(&[]);
    compiler.fn_builder.finalize();
    compiler.fn_builder.seal_all_blocks();
    compiler.fn_builder.finalize();
    let mut trap_sink = codegen::binemit::NullTrapSink {};
    let mut stack_map_sink = codegen::binemit::NullStackMapSink {};
    module
        .define_function(fn_main, &mut ctx, &mut trap_sink, &mut stack_map_sink)
        .map_err(to_other_err)?;

    // Link.
    module.finalize_definitions();
    println!("{}", ctx.func);

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
    str_val: &[u8],
    str_id: &mut usize,
) {
    let mut data_ctx = DataContext::new();
    data_ctx.define(str_val.to_owned().into_boxed_slice());

    let name = format!("str_{}", str_id);
    *str_id += 1;

    let id = module
        .declare_data(&name, Linkage::Export, false, false)
        .expect("Declaring a global string immediate.");
    module
        .define_data(id, &data_ctx)
        .expect("Defining a global string immediate.");

    data_map.insert(str_val.to_owned(), id);
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
        AstNode::For { ident, body, .. } => {
            declare_variable(fn_builder, var_map, ident, var_id);
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
        fn_builder.declare_var(var, types::I32);
    }
}

// -------------------------------------------------------------------------------------------------

struct Compiler<'a> {
    module: &'a mut JITModule,
    fn_builder: FunctionBuilder<'a>,
    data_map: HashMap<Vec<u8>, DataId>,
    var_map: HashMap<String, Variable>,
}

impl<'a> Compiler<'a> {
    fn compile_code(&mut self, program: &AstNode) -> Value {
        match program {
            AstNode::Literal(AstValue::Int(i)) => self.fn_builder.ins().iconst(types::I32, *i),
            AstNode::Identifier(name) => {
                let variable = self.var_map.get(name).expect("undefined variable");
                self.fn_builder.use_var(*variable)
            }
            AstNode::Call(name, args) => self.compile_call(name, args),
            AstNode::Assign(name, expr) => {
                let rhs_value = self.compile_code(expr);
                let variable = self.var_map.get(name).expect("undefined variable");
                self.fn_builder.def_var(*variable, rhs_value);
                rhs_value
            }
            AstNode::If {
                cond_expr,
                true_expr,
                false_expr,
            } => self.compile_if(cond_expr, true_expr, false_expr),
            AstNode::For {
                ident,
                first,
                last,
                body,
            } => self.compile_for(ident, *first, *last, body),

            _ => panic!("unhandled node: {:?}", program),
        }
    }

    // ---------------------------------------------------------------------------------------------

    fn compile_call(&mut self, name: &str, args: &[AstNode]) -> Value {
        if name == "print" {
            // At the moment, for this demo, the only function we do call is `print` and it takes a
            // single literal or an identifier referencing an int value.
            assert!(args.len() == 1);
            match &args[0] {
                AstNode::Literal(AstValue::Text(s)) => self.compile_print_str(s),
                AstNode::Literal(AstValue::Int(i)) => self.compile_print_int(*i),
                AstNode::Identifier(i) => self.compile_print_sym(i),

                _ => panic!("unexpected argument for print()!"),
            }
        } else {
            // Otherwise it's one of the binary operators.
            assert!(args.len() == 2);
            let lhs = self.compile_code(&args[0]);
            let rhs = self.compile_code(&args[1]);
            match name {
                "&&" => self.fn_builder.ins().band(lhs, rhs),
                "==" => {
                    let cmp_val = self.fn_builder.ins().icmp(IntCC::Equal, lhs, rhs);
                    self.fn_builder.ins().bint(types::I32, cmp_val)
                }
                "%" => self.fn_builder.ins().urem(lhs, rhs),

                _ => panic!("Unexpected function call: '{}'", name),
            }
        }
    }

    // ---------------------------------------------------------------------------------------------

    fn compile_if(
        &mut self,
        cond_expr: &AstNode,
        true_exprs: &[AstNode],
        false_exprs: &[AstNode],
    ) -> Value {
        let cond_val = self.compile_code(cond_expr);

        let true_block = self.fn_builder.create_block();
        let false_block = self.fn_builder.create_block();
        let final_block = self.fn_builder.create_block();

        // Jump to block depending on condition.
        self.fn_builder.ins().brz(cond_val, false_block, &[]);
        self.fn_builder.ins().jump(true_block, &[]);

        // Populate the true block, jump to final block at end.
        self.fn_builder.switch_to_block(true_block);
        self.fn_builder.seal_block(true_block);
        for expr in true_exprs {
            self.compile_code(expr);
        }
        self.fn_builder.ins().jump(final_block, &[]);

        // Populate the false block, jump to the final block at end.
        self.fn_builder.switch_to_block(false_block);
        self.fn_builder.seal_block(false_block);
        for expr in false_exprs {
            self.compile_code(expr);
        }
        self.fn_builder.ins().jump(final_block, &[]);

        // Switch to final block for rest of program.
        self.fn_builder.switch_to_block(final_block);
        self.fn_builder.seal_block(final_block);

        // Need to return a dummy null value.
        self.fn_builder.ins().iconst(types::I32, 0)
    }

    // ---------------------------------------------------------------------------------------------

    fn compile_for(&mut self, name: &str, first: i64, last: i64, body: &[AstNode]) -> Value {
        // Initialise the iterator.
        let variable = self.var_map.get(name).expect("undefined for-loop variable");
        let first_val = self.fn_builder.ins().iconst(types::I32, first);
        self.fn_builder.def_var(*variable, first_val);

        let cmp_block = self.fn_builder.create_block();
        let body_block = self.fn_builder.create_block();
        let final_block = self.fn_builder.create_block();

        self.fn_builder.ins().jump(cmp_block, &[]);

        // The comparison block compares the iterator to last.
        self.fn_builder.switch_to_block(cmp_block);
        let iter_var = self.fn_builder.use_var(*variable);
        let iter_is_lt =
            self.fn_builder
                .ins()
                .icmp_imm(IntCC::SignedLessThanOrEqual, iter_var, last);
        self.fn_builder.ins().brz(iter_is_lt, final_block, &[]);
        self.fn_builder.ins().jump(body_block, &[]);

        self.fn_builder.switch_to_block(body_block);
        for expr in body {
            self.compile_code(expr);
        }

        let inc_iter_var = self.fn_builder.ins().iadd_imm(iter_var, 1);
        let variable = self.var_map.get(name).expect("undefined for-loop variable");
        self.fn_builder.def_var(*variable, inc_iter_var);
        self.fn_builder.ins().jump(cmp_block, &[]);

        // Switch to final block for rest of program.
        self.fn_builder.switch_to_block(final_block);
        self.fn_builder.seal_block(cmp_block);
        self.fn_builder.seal_block(body_block);
        self.fn_builder.seal_block(final_block);

        // Need to return a dummy null value.
        self.fn_builder.ins().iconst(types::I32, 0)
    }

    // ---------------------------------------------------------------------------------------------

    fn compile_print_str(&mut self, str_val: &[u8]) -> Value {
        // int puts(const char* str)
        let mut sig = self.module.make_signature();
        let ptr_type = self.module.target_config().pointer_type();
        sig.params.push(AbiParam::new(ptr_type));
        sig.returns.push(AbiParam::new(types::I32));

        let libc_puts = self
            .module
            .declare_function("puts", Linkage::Import, &sig)
            .expect("Failed to declare `puts()`");
        let callee = self
            .module
            .declare_func_in_func(libc_puts, &mut self.fn_builder.func);

        let data_id = self.data_map.get(str_val).unwrap();
        let local_id = self
            .module
            .declare_data_in_func(*data_id, &mut self.fn_builder.func);

        let arg = self.fn_builder.ins().symbol_value(ptr_type, local_id);
        self.fn_builder.ins().call(callee, &[arg]);
        arg
    }

    fn compile_print_int(&mut self, int_val: i64) -> Value {
        let value = self.fn_builder.ins().iconst(types::I32, int_val);
        self.compile_print_int_value(value);
        value
    }

    fn compile_print_sym(&mut self, ident: &str) -> Value {
        let variable = self.var_map.get(ident).unwrap();
        let value = self.fn_builder.use_var(*variable);
        self.compile_print_int_value(value);
        value
    }

    fn compile_print_int_value(&mut self, value: Value) {
        // NOTE: This is a complete hack, because I want to get something working ASAP!
        //
        // We can only print positive integers, between 0 and 999.  To do that we print each digit
        // in turn.

        // int putchar(int c)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I32));
        sig.returns.push(AbiParam::new(types::I32));

        let libc_putchar = self
            .module
            .declare_function("putchar", Linkage::Import, &sig)
            .expect("Failed to declare `putchar()`");
        let callee = self
            .module
            .declare_func_in_func(libc_putchar, &mut self.fn_builder.func);

        let space = self.fn_builder.ins().iconst(types::I32, 32);
        let nl = self.fn_builder.ins().iconst(types::I32, 10);

        // First char, 100s column!
        let var_div_100 = self.fn_builder.ins().udiv_imm(value, 100);
        let var_0_mod_10 = self.fn_builder.ins().urem_imm(var_div_100, 10);
        let var_0_digit = self.fn_builder.ins().iadd_imm(var_0_mod_10, 48); // digit + '0'
        let var_0_is_z = self
            .fn_builder
            .ins()
            .icmp_imm(IntCC::Equal, var_0_mod_10, 0);
        let var_0_ch = self.fn_builder.ins().select(var_0_is_z, space, var_0_digit);
        self.fn_builder.ins().call(callee, &[var_0_ch]);

        // Second char, 10s column!
        let var_div_10 = self.fn_builder.ins().udiv_imm(value, 10);
        let var_1_mod_10 = self.fn_builder.ins().urem_imm(var_div_10, 10);
        let var_1_digit = self.fn_builder.ins().iadd_imm(var_1_mod_10, 48); // digit + '0'
        let var_1_is_z = self
            .fn_builder
            .ins()
            .icmp_imm(IntCC::Equal, var_1_mod_10, 0);
        let var_both_are_z = self.fn_builder.ins().band(var_0_is_z, var_1_is_z);
        let var_1_ch = self
            .fn_builder
            .ins()
            .select(var_both_are_z, space, var_1_digit);
        self.fn_builder.ins().call(callee, &[var_1_ch]);

        // Third char, 1s column!
        let var_2_mod_10 = self.fn_builder.ins().urem_imm(value, 10);
        let var_2_ch = self.fn_builder.ins().iadd_imm(var_2_mod_10, 48); // digit + '0'
        self.fn_builder.ins().call(callee, &[var_2_ch]);

        // Newline.
        self.fn_builder.ins().call(callee, &[nl]);
    }
}

// -------------------------------------------------------------------------------------------------
