use std::{collections::HashMap, path::Path};

use cafebabe::{
    attributes::AttributeData,
    bytecode::{ByteCode, Opcode},
};
use inkwell::{
    basic_block::BasicBlock,
    context::Context,
    memory_buffer::MemoryBuffer,
    module::{Linkage, Module},
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple},
    values::BasicValueEnum,
    OptimizationLevel,
};
fn make_std_module(c: &Context) -> Module {
    // let std = std::fs::read("./std.ll").unwrap();
    c.create_module_from_ir(MemoryBuffer::create_from_file(Path::new("./std.ll")).unwrap())
        .unwrap()
}
fn main() {
    let ctx = Context::create();
    let builder = ctx.create_builder();
    let class_module = ctx.create_module("class");
    // class_module.link_in_module(make_std_module(&ctx)).unwrap();
    let bytes = std::fs::read("./Main.class").unwrap();
    let class = cafebabe::parse_class(&bytes).unwrap();

    println!("{:#?}", class);
    let int32 = ctx.i32_type();
    let void = ctx.void_type();

    let stack_ty = {
        let arr = int32.array_type(1024);
        ctx.struct_type(&[arr.into(), int32.into()], false)
            .ptr_type(inkwell::AddressSpace::Generic)
    };

    let varstore_ty = {
        let arr = int32.array_type(1024);
        ctx.struct_type(&[arr.into()], false)
    };
    let std_stack_new = class_module.add_function(
        "stack_new",
        stack_ty.fn_type(&[], false),
        Some(Linkage::External),
    );
    let std_stack_push = class_module.add_function(
        "stack_push",
        void.fn_type(&[stack_ty.into(), int32.into()], false),
        Some(Linkage::External),
    );
    let std_stack_pop = class_module.add_function(
        "stack_pop",
        int32.fn_type(&[stack_ty.into()], false),
        Some(Linkage::External),
    );

    for method in &class.methods {
        println!("Method: {} {}", method.name, method.descriptor);
        let function = class_module.add_function(
            &method.name,
            ctx.i32_type()
                .fn_type(&[ctx.i32_type().into(), ctx.i32_type().into()], false),
            None,
        );
        let code = method
            .attributes
            .iter()
            .find(|a| {
                if let AttributeData::Code(_) = &a.data {
                    true
                } else {
                    false
                }
            })
            .unwrap();
        if let AttributeData::Code(c) = &code.data {
            let entry = ctx.append_basic_block(function, "entry");
            builder.position_at_end(entry);
            // region:stuff
            let stack = BasicValueEnum::PointerValue(
                builder
                    .build_call(std_stack_new, &[], "stack")
                    .try_as_basic_value()
                    .unwrap_left()
                    .into_pointer_value(),
            );
            let push = |t| builder.build_call(std_stack_push, &[stack, t], "stack_push");
            let pop = || {
                builder
                    .build_call(std_stack_pop, &[stack], "stack_pop")
                    .try_as_basic_value()
                    .unwrap_left()
            };
            // endregion:stuff
            let mut block_map = HashMap::<usize, BasicBlock>::new();
            let i = c.bytecode.as_ref().unwrap().opcodes.iter().peekable();
            let mut should_branch_previous_to_current = true;
            let locals = (0..=3)
                .map(|i| builder.build_alloca(ctx.i32_type(), &format!("local{}", i)))
                .collect::<Vec<_>>();
            for p in 1..function.count_params()+1 { // TODO: Make this automatic and use function param count
                let ptr = locals.get(p as usize).unwrap();
                builder.build_store(*ptr, function.get_nth_param(p-1).unwrap());
            }
            for (o, u) in i.clone() {
                let bb = ctx.append_basic_block(function, &format!("Instruction::{}", o));
                block_map.insert(*o, bb);
            }
            for (offset, opcode) in i {
                let bb = *block_map.get(offset).unwrap();
                // Branch previous instruction to current instruction.
                if should_branch_previous_to_current {
                    builder.build_unconditional_branch(bb);
                } else {
                    // Insert custom branching code here
                }

                should_branch_previous_to_current = match opcode {
                    Opcode::IfIcmpne(_) => false,
                    Opcode::Ireturn => false,
                    _ => true,
                };
                // builder.build_unconditional_branch(bb);

                builder.position_at_end(bb);
                match opcode {
                    Opcode::Iload(n) => {
                        // let table: [u64; 3] = [0, 1, 5];
                        println!("Locals {:#?}", locals);
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "iload");
                        push(l);
                    }
                    Opcode::Aload(_) => {}
                    Opcode::Invokespecial(_) => {}
                    Opcode::Return => {
                        let v = pop();
                        builder.build_return(Some(&v));
                    }
                    Opcode::Ireturn => {
                        let v = pop();
                        builder.build_return(Some(&v));
                    }
                    Opcode::IfIcmpne(joff) => {
                        let long_branch = offset + (*joff as usize);
                        let longbb = block_map.get(&long_branch).unwrap();
                        let shortbb = block_map.get(&(offset + 3)).unwrap();

                        let lhs = pop().into_int_value();
                        let rhs = pop().into_int_value();

                        let cmp = builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            lhs,
                            rhs,
                            "IHATEUNITEDNATIONS",
                        );
                        builder.build_conditional_branch(cmp, *shortbb, *longbb);
                    }
                    Opcode::Iconst1 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(1u64, false),
                        ));
                    }
                    Opcode::Sipush(n) => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(*n as u64, false),
                        ));
                    }
                    _ => panic!("0x{:?} is not implemented yet!", opcode),
                }
                // Normal code
            }
        }

        // println!("Code Data {:#?}", c);
        // let until_ret = until_terminator(0, &c.bytecode.as_ref().unwrap());
        // println!("Until ret {:#?}", until_ret);
        // for (offset, instruction) in &c.bytecode.as_ref().unwrap().opcodes {
        //     match instruction {
        //         cafebabe::bytecode::Opcode::IfIcmpne(notsure) => {
        //             let o = offset + (*notsure as usize);
        //             let jmp = c.bytecode.as_ref().unwrap().get_opcode_index(o).unwrap();
        //             // println!(
        //             // "Jmp to {:?}",
        //             // c.bytecode.as_ref().unwrap().opcodes.get(jmp).unwrap().1
        //             // );
        //             // println!("Not sure {}", notsure);
        //         }
        //         _ => println!("Not implemented"),
        //     }
        // }
        // bcode.get_opcode_index(offset)
    }
    // class_module.verify().unwrap();
    class_module.print_to_stderr();
    match class_module.verify() {
        Ok(_) => {
            // let e = class_module.create_execution_engine().unwrap();
            // e.run_function(function, args)
            Target::initialize_all(&InitializationConfig {
                ..Default::default()
            });
            let target_triple = TargetMachine::get_default_triple();
            let target = Target::from_triple(&target_triple).unwrap();
            let t = target
                .create_target_machine(
                    &target_triple,
                    &TargetMachine::get_host_cpu_name().to_string(),
                    &TargetMachine::get_host_cpu_features().to_string(),
                    OptimizationLevel::None,
                    RelocMode::Default,
                    CodeModel::Default,
                )
                .unwrap();
            t.write_to_file(
                &class_module,
                inkwell::targets::FileType::Assembly,
                Path::new("./out.s"),
            )
            .unwrap();
            t.write_to_file(
                &class_module,
                inkwell::targets::FileType::Object,
                Path::new("./out.o"),
            )
            .unwrap();
            class_module.print_to_file("module.ll").unwrap();
        }

        Err(e) => {
            println!("Could not validate: {}", e.to_string());
        }
    }
}

// fn until_terminator<'a>(start: usize, b: &'a ByteCode) -> ControlTree<'a> {
//     println!("Testing bytecode {:#?}", b);
//     let mut i = b.opcodes.iter();
//     let mut contains_branch = false;
//     for _ in 0..start {
//         i.next().unwrap();
//     }
//     let until_ret = i
//         .take_while(|(_offset, o)| if let Opcode::Ireturn = o { false } else { true })
//         .collect::<Vec<_>>();
//     for (offset, o) in &until_ret {
//         if let Opcode::IfIcmpne(false_offset) = o {
//             contains_branch = true;
//             let false_absolute = offset + (false_offset as usize );
//         }
//     }
//     match contains_branch {
//         true => ControlTree::SomeComparison
//     }
//     // ControlTree::Normal(until_ret)
// }
// #[derive(Debug)]
// enum ControlTree<'a> {
//     Normal(Vec<&'a (usize, Opcode<'a>)>),
//     SomeComparison {
//         next: Box<ControlTree<'a>>,
//         jmp: Box<ControlTree<'a>>,
//     },
//     Null,
// }
