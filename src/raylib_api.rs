use std::ffi::CString;

use magnus::{function, prelude::*, Error, IntoValue, RArray, Ruby, Value};
use raylib::ffi;

#[path = "raylib_api_generated.rs"]
mod generated;

fn argument(message: impl Into<String>) -> Error {
    Error::new(Ruby::get().unwrap().exception_arg_error(), message.into())
}

fn sequence(value: Value, count: usize) -> Result<Vec<Value>, Error> {
    let array = if let Some(array) = RArray::from_value(value) {
        array
    } else {
        value.funcall::<_, _, RArray>("to_a", ())?
    };
    if array.len() != count {
        return Err(argument(format!(
            "Expected {count} fields, got {}",
            array.len()
        )));
    }
    Ok(array.into_iter().collect())
}

fn address(value: Value) -> Result<usize, Error> {
    if value.is_nil() {
        return Ok(0);
    }
    usize::try_convert(value).or_else(|_| value.funcall("address", ()))
}

enum StringArgument {
    Owned(CString),
    Address(usize),
}

impl StringArgument {
    fn pointer(&self) -> *const std::ffi::c_char {
        match self {
            Self::Owned(text) => text.as_ptr(),
            Self::Address(address) => *address as _,
        }
    }
}

fn string_argument(value: Value) -> Result<StringArgument, Error> {
    if let Some(text) = magnus::RString::from_value(value) {
        let text = String::try_convert(text.as_value())?;
        CString::new(text)
            .map(StringArgument::Owned)
            .map_err(|_| argument("C strings cannot contain NUL bytes"))
    } else {
        address(value).map(StringArgument::Address)
    }
}

fn arity(args: &[Value], count: usize) -> Result<(), Error> {
    if args.len() == count {
        Ok(())
    } else {
        Err(argument(format!(
            "Expected {count} arguments, got {}",
            args.len()
        )))
    }
}

fn invoke(name: String, args: RArray) -> Result<Value, Error> {
    crate::game::need_game()?;
    generated::call(
        &Ruby::get().unwrap(),
        &name,
        &args.into_iter().collect::<Vec<_>>(),
    )
}

pub fn register(ruby: &Ruby) -> Result<(), Error> {
    let raylib = ruby.define_module("Raylib")?;
    let native = raylib.define_module("Native")?;
    native.define_singleton_method("__invoke", function!(invoke, 2))?;
    native.const_set("SCHEMA_JSON", include_str!("ruby/raylib_api.json"))?;
    let _: Value = ruby.eval(include_str!("ruby/raylib_native.rb"))?;
    Ok(())
}
