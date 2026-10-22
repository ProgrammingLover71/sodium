use crate::parser::ast;

mod error;
mod lexer;
mod parser;
mod sil;
mod silgen;
mod codegen;

use codegen::Allocator;

use std::mem;

// let mut b = sil::ILBuilder::new(false);

// let foo_id = b.define_function(
// 	String::from("foo"),
// 	vec![],
// 	sil::Type::Int64
// );
// b.set_current_function(foo_id);

// let foo_bb0 = b.make_entry_block(foo_id);

// let res: Result<(), error::Error> = {
// 	let v0 = b.const_i64(foo_bb0, 5)?;
// 	let v1 = b.const_i64(foo_bb0, 7)?;
// 	let v2 = b.add(foo_bb0, v0, v1)?;
// 	let v3 = b.call(foo_bb0, v2, vec![v2])?;
// 	b.ret(foo_bb0, v3)?;

// 	Ok(())
// };

// let foo = b.get_function(foo_id);

// println!("{}", foo?);

// Ok(())

fn main() -> Result<(), error::Error> {
    let src = String::from("return 63 + 45 + 12");

    let lexer = lexer::Lexer::new(src);

    let mut arena = ast::NodeArena::new();
    let parser = parser::Parser::new(lexer, &mut arena);
    let program = parser.program()?;

    println!("{:?}", program);

    let mut sil_gen = silgen::SILGenerator::new(&program, false);
    let (ilb, f_main_id) = sil_gen.compile()?;

    let f_main = ilb.get_function(f_main_id)?;
    println!("{}", f_main);

	let mut alloc = codegen::DefaultAllocator::new();
    let mut cgen = codegen::Codegen::<codegen::DefaultAllocator>::new(&mut alloc);
    let (main_buf, main_off) = cgen.compile_function(f_main).unwrap();
    let main_p = main_buf.ptr(main_off);

	let code = main_buf.as_ref();

	for (i, byte) in code.iter().enumerate() {
		print!("{:02X} ", byte);

		if (i + 1) % 16 == 0 {
			println!();
		}
	}

	println!();

    let main: extern "C" fn() -> i64 = unsafe { mem::transmute(main_p) };
    println!("{}", main());

    Ok(())
}
