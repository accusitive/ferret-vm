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
    passes::PassManager,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple},
    values::{BasicValueEnum, GenericValue, IntValue},
    OptimizationLevel,
};
mod util;

fn main() {
    let ctx = Context::create();
    let builder = ctx.create_builder();
    let class_module = ctx.create_module("class");
    let fpm = PassManager::create(&class_module);

    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.add_gvn_pass();
    fpm.add_cfg_simplification_pass();
    fpm.add_basic_alias_analysis_pass();
    fpm.add_promote_memory_to_register_pass();
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    // fpm.add
    fpm.initialize();
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
        let function_type = util::parse_method_type(&ctx, &method.descriptor);
        let function = match class_module.get_function(&method.name) {
            Some(f) => f,
            None => class_module.add_function(&method.name, function_type.1, None),
        };

        let code = method.attributes.iter().find(|a| {
            if let AttributeData::Code(_) = &a.data {
                true
            } else {
                false
            }
        });
        if code.is_none() {
            continue;
        }
        let code = code.unwrap();

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
            for p in 1..function.count_params() + 1 {
                // TODO: Make this automatic and use function param count
                let ptr = locals.get(p as usize).unwrap();
                builder.build_store(*ptr, function.get_nth_param(p - 1).unwrap());
            }
            for (o, u) in i.clone() {
                let bb = ctx.append_basic_block(function, &format!("Ox{:x}-{:?}", o, u));
                block_map.insert(*o, bb);
            }
            for (offset, opcode) in i {
                let bb = *block_map.get(offset).unwrap();
                // Branch previous instruction to current instruction.
                if should_branch_previous_to_current {
                    builder.build_unconditional_branch(bb);
                }

                should_branch_previous_to_current = match opcode {
                    Opcode::IfIcmpeq(_) => false,
                    Opcode::IfIcmpge(_) => false,
                    Opcode::IfIcmple(_) => false,
                    Opcode::IfIcmpgt(_) => false,
                    Opcode::IfIcmplt(_) => false,
                    Opcode::IfIcmpne(_) => false,

                    Opcode::Ireturn => false,
                    _ => true,
                };
                // builder.build_unconditional_branch(bb);
                let impl_branching = |predicate, offset, joff: &_| {
                    let long_branch = offset + (*joff as usize);
                    let longbb = block_map.get(&long_branch).unwrap();
                    let shortbb = block_map.get(&(offset + 3)).unwrap();

                    let rhs = pop().into_int_value();
                    let lhs = pop().into_int_value();

                    let cmp = builder.build_int_compare(predicate, lhs, rhs, "intcompare");

                    builder.build_conditional_branch(cmp, *longbb, *shortbb);
                };
                builder.position_at_end(bb);
                match opcode {
                    Opcode::Iload(n) => {
                        // let table: [u64; 3] = [0, 1, 5];
                        println!("Locals {:#?}", locals);
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "aload");
                        push(l);
                    }
                    Opcode::Aload(n) => {
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "iload");
                        push(l);
                    }
                    Opcode::Invokespecial(_) => {}
                    Opcode::Return => {
                        builder.build_return(None);
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
                            inkwell::IntPredicate::NE,
                            lhs,
                            rhs,
                            "intcompare",
                        );

                        builder.build_conditional_branch(cmp, *longbb, *shortbb);
                    }

                    Opcode::IfIcmpeq(n) => {
                        impl_branching(inkwell::IntPredicate::EQ, offset, n);
                    }
                    Opcode::IfIcmpge(n) => impl_branching(inkwell::IntPredicate::SGE, offset, n),
                    Opcode::IfIcmpgt(n) => impl_branching(inkwell::IntPredicate::SGT, offset, n),
                    Opcode::IfIcmple(n) => impl_branching(inkwell::IntPredicate::SLE, offset, n),
                    Opcode::IfIcmplt(n) => impl_branching(inkwell::IntPredicate::SLT, offset, n),
                    // region: consts
                    Opcode::Iconst1 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(1u64, false),
                        ));
                    }
                    Opcode::Iconst2 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(2u64, false),
                        ));
                    }
                    Opcode::Iconst3 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(3u64, false),
                        ));
                    }
                    Opcode::Iconst4 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(4u64, false),
                        ));
                    }
                    Opcode::Iconst5 => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(5u64, false),
                        ));
                    }
                    // endregion: consts
                    Opcode::Sipush(n) => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(*n as u64, false),
                        ));
                    }
                    Opcode::Bipush(n) => {
                        push(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(*n as u64, false),
                        ));
                    }
                    Opcode::Invokevirtual(member) => {
                        let a = util::parse_method_type(&ctx, &member.name_and_type.descriptor);
                        let f = match class_module.get_function(&member.name_and_type.name) {
                            Some(m) => m,
                            None => {
                                class_module.add_function(&member.name_and_type.name, a.1, None)
                            }
                        };
                        println!("Member {:?} wants {} rgs", member, a.0);
                        let popped = (0..a.0).map(|i| pop()).collect::<Vec<_>>();
                        let object_ref = pop();

                        let result = builder.build_call(f, &popped, "call");
                        push(result.try_as_basic_value().unwrap_left());
                    }
                    Opcode::Imul => {
                        let lhs = pop().into_int_value();
                        let rhs = pop().into_int_value();
                        let product = builder.build_int_mul(lhs, rhs, "imul");
                        push(inkwell::values::BasicValueEnum::IntValue(product));
                    }
                    Opcode::Iadd => {
                        let lhs = pop().into_int_value();
                        let rhs = pop().into_int_value();
                        let sum = builder.build_int_add(lhs, rhs, "iadd");
                        push(inkwell::values::BasicValueEnum::IntValue(sum));
                    }
                    Opcode::Isub => {
                        let lhs = pop().into_int_value();
                        let rhs = pop().into_int_value();
                        let sum = builder.build_int_sub(lhs, rhs, "isub");
                        push(inkwell::values::BasicValueEnum::IntValue(sum));
                    }
                    Opcode::Pop => {
                        pop();
                    }
                    _ => panic!("0x{:?} is not implemented yet!", opcode),
                }
                // Normal code
            }
        }
        if function.verify(true) {
            // fpm.run_on(&function);
        }
    }
    class_module.print_to_stderr();
    match class_module.verify() {
        Ok(_) => {
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
                    OptimizationLevel::Aggressive,
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

            // let engine = class_module.create_jit_execution_engine(OptimizationLevel::Aggressive).unwrap();
            // unsafe {
            //     // engine.run_function(
            //     //     class_module.get_function("add").unwrap(),
            //     //     &[&ctx.i32_type().create_generic_value(2, false),&ctx.i32_type().create_generic_value(2, false)],
            //     // );
            //     let fp = engine.get_function::<unsafe extern "C" fn(i32,i32) -> i32>("add").unwrap();
            //     fp.call(2,4);
            //     // let f = std::mem::transmute::<usize, extern "C" fn(i32, i32) -> i32>(fp);
            //     // let z =  f(2,2);
            //     // println!("Output of F(2,2) -> {}",z);
            // };
        }

        Err(e) => {
            println!("Could not validate: {}", e.to_string());
            panic!();
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
