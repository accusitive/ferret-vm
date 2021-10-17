use inkwell::{
    context::Context,
    types::{AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, FunctionType},
};
use jni::signature::JavaType;
pub struct MethodType<'a> {
    pub fnty: FunctionType<'a>,
    pub parameters: Vec<BasicTypeEnum<'a>>,
}
pub fn jnity_to_llvm_ty<'a>(ctx: &'a Context, j: &JavaType) -> AnyTypeEnum<'a> {
    match j {
        jni::signature::JavaType::Primitive(p) => match p {
            jni::signature::Primitive::Boolean => ctx.bool_type().as_any_type_enum(),
            jni::signature::Primitive::Byte | jni::signature::Primitive::Char => {
                ctx.i8_type().as_any_type_enum()
            }
            jni::signature::Primitive::Double => ctx.f64_type().as_any_type_enum(),
            jni::signature::Primitive::Float => ctx.f32_type().as_any_type_enum(),
            jni::signature::Primitive::Int => ctx.i32_type().as_any_type_enum(),
            jni::signature::Primitive::Long => ctx.i64_type().as_any_type_enum(),
            jni::signature::Primitive::Short => ctx.i16_type().as_any_type_enum(),
            jni::signature::Primitive::Void => ctx.void_type().as_any_type_enum(),
        },
        jni::signature::JavaType::Object(_) => ctx.i32_type().as_any_type_enum(),
        jni::signature::JavaType::Array(_) => todo!(),
        jni::signature::JavaType::Method(_) => todo!(),
    }
}
pub fn parse_method_type<T: ToString>(ctx: &Context, d: T) -> MethodType {
    let s = jni::signature::TypeSignature::from_str(d.to_string()).unwrap();
    // println!("SIG {:?}", s);
    // BasicTypeEnum::
    // ret.fn_type(param_types, is_var_args)
    let params = &s
        .args
        .iter()
        .map(|a| jnity_to_llvm_ty(&ctx, a).try_into().unwrap())
        .collect::<Vec<_>>();
        println!("Params as parsed: {:#?}", params);

    let ret = jnity_to_llvm_ty(&ctx, &s.ret);
    if let AnyTypeEnum::VoidType(v) = ret {
        // let k: BasicTypeEnum = v;

        let t = v.fn_type(&params, false);
        // (params.len(), t)
        MethodType {
            fnty: t,
            parameters: params.to_vec(),
        }
    } else {
        let t: BasicTypeEnum = ret.try_into().unwrap();
        let f = t.fn_type(&params, false);

        MethodType {
            fnty: f,
            parameters: params.to_vec(),
        }
    }
}
