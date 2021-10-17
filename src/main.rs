use std::{collections::HashMap, path::Path};

use cafebabe::{attributes::AttributeData, bytecode::Opcode};
use inkwell::{
    basic_block::BasicBlock,
    context::Context,
    module::Linkage,
    passes::{PassManager, PassManagerBuilder, PassRegistry},
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    values::BasicValueEnum,
    IntPredicate, OptimizationLevel,
};
mod util;

fn main() {
    // let _exit = std::process::Command::new("/usr/bin/javac")
    //     .arg("Main.java")
    //     .spawn()
    //     .unwrap()
    //     .wait()
    //     .unwrap();

    let ctx = Context::create();
    let builder = ctx.create_builder();
    let class_module = ctx.create_module("class");

    let fpm = PassManager::create(&class_module);
    let pass_builder = PassManagerBuilder::create();

    pass_builder.set_optimization_level(OptimizationLevel::Aggressive);
    pass_builder.set_size_level(0);
    pass_builder.set_inliner_with_threshold(1);
    pass_builder.set_disable_unit_at_a_time(false);
    pass_builder.set_disable_unroll_loops(false);
    pass_builder.set_disable_simplify_lib_calls(false);

    let pass_registry = PassRegistry::get_global();
    pass_registry.initialize_core();
    pass_registry.initialize_transform_utils();
    pass_registry.initialize_scalar_opts();
    pass_registry.initialize_obj_carc_opts();
    pass_registry.initialize_vectorization();
    pass_registry.initialize_inst_combine();
    pass_registry.initialize_ipo();
    pass_registry.initialize_instrumentation();
    pass_registry.initialize_analysis();
    pass_registry.initialize_ipa();
    pass_registry.initialize_codegen();
    pass_registry.initialize_target();
    pass_registry.initialize_aggressive_inst_combiner();
    pass_builder.populate_function_pass_manager(&fpm);

    // fpm.add
    fpm.initialize();

    // class_module.link_in_module(make_std_module(&ctx)).unwrap();
    let bytes = std::fs::read("./Main.class").unwrap();
    let class = cafebabe::parse_class(&bytes).unwrap();

    // println!("{:#?}", class);
    // ctx.type
    let int = ctx.i32_type();
    let float = ctx.f32_type();

    let void = ctx.void_type();
    // region: STD
    // init
    let stack_ty = {
        let arr = int.array_type(1024);
        ctx.struct_type(&[arr.into(), int.into()], false)
            .ptr_type(inkwell::AddressSpace::Generic)
    };

    let varstore_ty = {
        let arr = int.array_type(1024);
        ctx.struct_type(&[arr.into()], false)
            .ptr_type(inkwell::AddressSpace::Generic)
    };
    let varstore_new = class_module.add_function(
        "varstore_new",
        varstore_ty.fn_type(&[], false),
        Some(Linkage::External),
    );

    let varstore_set = class_module.add_function(
        "varstore_set",
        void.fn_type(
            &[
                inkwell::types::BasicTypeEnum::PointerType(varstore_ty),
                int.into(),
                int.into(),
            ],
            false,
        ),
        Some(Linkage::External),
    );
    let varstore_get = class_module.add_function(
        "varstore_get",
        int.fn_type(
            &[
                inkwell::types::BasicTypeEnum::PointerType(varstore_ty),
                int.into(),
            ],
            false,
        ),
        Some(Linkage::External),
    );
    let std_stack_new = class_module.add_function(
        "stack_new",
        stack_ty.fn_type(&[], false),
        Some(Linkage::External),
    );
    // int
    let std_stack_pushi = class_module.add_function(
        "stack_pushi",
        void.fn_type(&[stack_ty.into(), int.into()], false),
        Some(Linkage::External),
    );
    let std_stack_popi = class_module.add_function(
        "stack_popi",
        int.fn_type(&[stack_ty.into()], false),
        Some(Linkage::External),
    );
    // float
    let std_stack_pushf = class_module.add_function(
        "stack_pushf",
        void.fn_type(&[stack_ty.into(), float.into()], false),
        Some(Linkage::External),
    );
    let std_stack_popf = class_module.add_function(
        "stack_popf",
        float.fn_type(&[stack_ty.into()], false),
        Some(Linkage::External),
    );
    // endregion: STD
    for method in &class.methods {
        // println!("Method: {} {}", method.name, method.descriptor);
        let function_type = util::parse_method_type(&ctx, &method.descriptor);
        let function = match class_module.get_function(&method.name) {
            Some(f) => f,
            None => class_module.add_function(&method.name, function_type.fnty, None),
        };

        let code = method.attributes.iter().find(|a| {
            if let AttributeData::Code(_) = &a.data {
                true
            } else {
                false
            }
        });
        // Native functions dont have a code attribute
        if code.is_none() {
            println!("Note: Method {} doesnt have a code attribute", method.name);
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
            let varstore = BasicValueEnum::PointerValue(
                builder
                    .build_call(varstore_new, &[], "varstore")
                    .try_as_basic_value()
                    .unwrap_left()
                    .into_pointer_value(),
            );
            // TODO: one function and have it automatically determine what std functioon it should call
            let pushi = |t| builder.build_call(std_stack_pushi, &[stack, t], "stack_pushi");
            let popi = || {
                builder
                    .build_call(std_stack_popi, &[stack], "stack_popi")
                    .try_as_basic_value()
                    .unwrap_left()
            };
            let pushf = |t| builder.build_call(std_stack_pushf, &[stack, t], "stack_pushf");
            let popf = || {
                builder
                    .build_call(std_stack_popf, &[stack], "stack_popf")
                    .try_as_basic_value()
                    .unwrap_left()
            };
            // endregion:stuff
            let mut block_map = HashMap::<usize, BasicBlock>::new();

            let i = c.bytecode.as_ref().unwrap().opcodes.iter().peekable();
            let mut should_branch_previous_to_current = true;
            let locals = (0..c.max_locals)
                .map(|i| builder.build_alloca(int, &format!("local{}", i)))
                .collect::<Vec<_>>();
            for p in 1..function.count_params() + 1 {
                let ptr = locals.get(p as usize).unwrap();
                let value = function.get_nth_param(p - 1).unwrap();
                let value_as_int = builder.build_bitcast(value, int, "bitcast_to_int");
                builder.build_store(*ptr, value_as_int);
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
                let impl_branching = |predicate, offset, joff: &_| {
                    let long_branch = offset + (*joff as usize);
                    let longbb = block_map.get(&long_branch).unwrap();
                    let shortbb = block_map.get(&(offset + 3)).unwrap();

                    let rhs = popi().into_int_value();
                    let lhs = popi().into_int_value();

                    let cmp = builder.build_int_compare(predicate, lhs, rhs, "intcompare");

                    builder.build_conditional_branch(cmp, *longbb, *shortbb);
                };
                builder.position_at_end(bb);
                match opcode {
                    Opcode::Iload(n) => {
                        println!("Locals {:#?}", locals);
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "iload");
                        pushi(l);
                    }
                    Opcode::Fload(n) => {
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "fload");
                        pushi(l);
                    }
                    Opcode::Aload(n) => {
                        let l = builder.build_load(*locals.get(*n as usize).unwrap(), "aload");
                        pushi(l);
                    }
                    Opcode::Fstore(n) => {
                        let fvalue = popf();
                        let value = builder.build_bitcast(fvalue, int, "bitcast_to_int");
                        let local = locals.get(*n as usize).unwrap();
                        builder.build_store(*local, value);
                    }
                    Opcode::Istore(n) => {
                        let value = popi();
                        let local = locals.get(*n as usize).unwrap();
                        builder.build_store(*local, value);
                    }
                    Opcode::Invokespecial(_) => {}
                    Opcode::Return => {
                        builder.build_return(None);
                    }
                    Opcode::Ireturn => {
                        let v = popi();
                        builder.build_return(Some(&v));
                    }
                    Opcode::Freturn => {
                        let v = popf();
                        builder.build_return(Some(&v));
                    }
                    Opcode::IfIcmpne(n) => impl_branching(IntPredicate::NE, offset, n),
                    Opcode::IfIcmpeq(n) => impl_branching(IntPredicate::EQ, offset, n),
                    Opcode::IfIcmpge(n) => impl_branching(IntPredicate::SGE, offset, n),
                    Opcode::IfIcmpgt(n) => impl_branching(IntPredicate::SGT, offset, n),
                    Opcode::IfIcmple(n) => impl_branching(IntPredicate::SLE, offset, n),
                    Opcode::IfIcmplt(n) => impl_branching(IntPredicate::SLT, offset, n),
                    // region: consts
                    Opcode::Iconst1 => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(1u64, false),
                        ));
                    }
                    Opcode::Iconst2 => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(2u64, false),
                        ));
                    }
                    Opcode::Iconst3 => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(3u64, false),
                        ));
                    }
                    Opcode::Iconst4 => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(4u64, false),
                        ));
                    }
                    Opcode::Iconst5 => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(5u64, false),
                        ));
                    }
                    // endregion: consts
                    Opcode::Sipush(n) => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(*n as u64, false),
                        ));
                    }
                    Opcode::Bipush(n) => {
                        pushi(BasicValueEnum::IntValue(
                            ctx.i32_type().const_int(*n as u64, false),
                        ));
                    }
                    Opcode::Invokevirtual(member) => {
                        let a = util::parse_method_type(&ctx, &member.name_and_type.descriptor);
                        let f = match class_module.get_function(&member.name_and_type.name) {
                            Some(m) => m,
                            None => {
                                class_module.add_function(&member.name_and_type.name, a.fnty, None)
                            }
                        };
                        println!("Member {:?} wants {} args", member, a.parameters.len());
                        let popped = (0..a.parameters.len()).map(|_| popi()).collect::<Vec<_>>();
                        let _object_ref = popi();

                        let result = builder.build_call(f, &popped, "call");
                        pushi(result.try_as_basic_value().unwrap_left());
                    }
                    Opcode::Imul => {
                        let lhs = popi().into_int_value();
                        let rhs = popi().into_int_value();
                        let product = builder.build_int_mul(lhs, rhs, "imul");
                        pushi(inkwell::values::BasicValueEnum::IntValue(product));
                    }
                    Opcode::Iadd => {
                        let lhs = popi().into_int_value();
                        let rhs = popi().into_int_value();
                        let sum = builder.build_int_add(lhs, rhs, "iadd");
                        pushi(inkwell::values::BasicValueEnum::IntValue(sum));
                    }
                    Opcode::Fadd => {
                        let lhs = popf().into_float_value();
                        let rhs = popf().into_float_value();
                        let sum = builder.build_float_add(lhs, rhs, "fadd");
                        pushf(inkwell::values::BasicValueEnum::FloatValue(sum));
                    }
                    Opcode::Isub => {
                        let rhs = popi().into_int_value();
                        let lhs = popi().into_int_value();
                        let sum = builder.build_int_sub(lhs, rhs, "isub");
                        pushi(inkwell::values::BasicValueEnum::IntValue(sum));
                    }
                    Opcode::Pop => {
                        popi();
                    }

                    _ => panic!("0x{:?} is not implemented yet!", opcode),
                }
                // Normal code (???)
            }
        }
        if function.verify(true) {
            fpm.run_on(&function);
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
        }

        Err(e) => {
            println!("Could not validate: {}", e.to_string());
            panic!();
        }
    }
}
