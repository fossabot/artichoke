#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

// Tests derived from mrusty @ 1.0.0
// <https://github.com/anima-engine/mrusty/tree/v1.0.0>

// mrusty. mruby safe bindings for Rust
// Copyright (C) 2016  Dragoș Tiselice
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The tests for crate `mruby-sys` defined in this module serve as an
//! implementation guide and API examples for the `mruby` higher-level
//! Rust bindings.
//!
//! ## Implementation Notes
//!
//! ### Strings
//!
//! There are two ways to pass Rust strings across an FFI boundary:
//!
//! - Call `as_ptr` on a `&str` and pass the length of the `&str`. This does
//!   not create a NUL-terminated ("\0") *char. An mruby function that has this
//!   API is `mrb_load_nstring_cxt`.
//! - Create a CString from a `&str` for a traditional *char C string. This
//!   creates a NUL-terminated ("\0") *char. An mruby functino that has this API
//!   is `mrb_define_class`.
//!
//! ### Exceptions
//!
//! If a Rust function that manipulates mruby state can raise a Ruby exception,
//! calling the function via `mrb_protect` executes the function surrounded by
//! a try catch and will return the result of the Rust function unless the code
//! raises, in which case it returns the exception.
//!
//! `mrb_protect` is useful in the context of executing untrusted code. Wrapping
//! all eval'd code in a rust function which is called via `mrb_protect`
//! ensures that code exits cleanly and we can report runtime errors to the
//! caller.

use std::ffi::{CStr, CString};

use super::*;

const TEST_CLASS: &str = "TestClass";
const TEST_MODULE: &str = "TestModule";

#[test]
fn open_close() {
    unsafe {
        let mrb = mrb_open();

        mrb_close(mrb);
    }
}

#[test]
fn symbol_to_string() {
    unsafe {
        let mrb = mrb_open();

        let name = CString::new(":symbol").expect(":symbol literal");
        let literal = name.as_ptr() as *const i8;
        let symbol = mrb_intern_cstr(mrb, literal);

        let s = mrb_sym2name(mrb, symbol) as *const i8;
        let s = CStr::from_ptr(s).to_str().expect(":symbol name").clone();
        assert_eq!(s, r#"":symbol""#);

        let mut s = mrb_sym2str(mrb, symbol);
        assert_eq!(s.tt, mrb_vtype_MRB_TT_STRING);
        let s = mrb_string_value_cstr(mrb, &mut s) as *const i8;
        let s = CStr::from_ptr(s).to_str().expect(":symbol.to_s").clone();
        assert_eq!(s, ":symbol");

        mrb_close(mrb);
    }
}

#[test]
fn define_method() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new(TEST_CLASS).unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        extern "C" fn rust__mruby__test_class__method__value(
            _mrb: *mut mrb_state,
            _slf: mrb_value,
        ) -> mrb_value {
            // TODO write extension code to expose inline function
            // `mrb_fixnum_value`.
            mrb_value {
                value: mrb_value__bindgen_ty_1 { i: 2 },
                tt: mrb_vtype_MRB_TT_FIXNUM,
            }
        }

        let new_method_str = CString::new("value").unwrap();

        mrb_define_method(
            mrb,
            new_class,
            new_method_str.as_ptr(),
            Some(rust__mruby__test_class__method__value),
            0,
        );

        let code = "TestClass.new.value";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn class_defined() {
    unsafe {
        let mrb = mrb_open();

        let obj_str = CString::new("Object").unwrap();
        let kernel_str = CString::new("Kernel").unwrap();
        let unknown_str = CString::new(TEST_CLASS).unwrap();

        assert_eq!(mrb_class_defined(mrb, obj_str.as_ptr()), 1_u8);
        assert_eq!(mrb_class_defined(mrb, kernel_str.as_ptr()), 1_u8);
        assert_eq!(mrb_class_defined(mrb, unknown_str.as_ptr()), 0_u8);

        mrb_close(mrb);
    }
}

#[test]
fn class_name() {
    unsafe {
        let mrb = mrb_open();

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new(TEST_CLASS).unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        let kernel_str = CString::new("Kernel").unwrap();
        let kernel = mrb_module_get(mrb, kernel_str.as_ptr());

        let name = mrb_class_name(mrb, new_class);

        assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "TestClass");

        let name = mrb_class_name(mrb, kernel);

        assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "Kernel");

        mrb_close(mrb);
    }
}

#[test]
fn class_value() {
    unsafe {
        let mrb = mrb_open();

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        // TODO: implement `mrb_ext_class_value(obj_class)`
        let obj_class = mrb_value {
            value: mrb_value__bindgen_ty_1 {
                p: obj_class as *mut std::ffi::c_void,
            },
            tt: mrb_vtype_MRB_TT_CLASS,
        };

        let to_s_str = CString::new("to_s").unwrap();
        let args = &[];

        let sym = mrb_intern(mrb, to_s_str.as_ptr(), 4usize);

        let result = mrb_funcall_argv(mrb, obj_class, sym, 0, args.as_ptr());

        let s = mrb_str_to_cstr(mrb, result) as *const i8;
        assert_eq!(result.tt, mrb_vtype_MRB_TT_STRING);
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "Object");

        mrb_close(mrb);
    }
}

#[test]
fn nil_class() {
    unsafe {
        let mrb = mrb_open();

        let nil = CString::new("NilClass").unwrap();
        let nil_class = mrb_class_get(mrb, nil.as_ptr() as *const i8);

        let name = mrb_class_name(mrb, nil_class);

        assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "NilClass");

        mrb_close(mrb);
    }
}

#[test]
fn nil_class_eval() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let code = "nil.class";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_CLASS);
        let s = mrb_class_name(mrb, result.value.p as *mut ffi::RClass);
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "NilClass");

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn nil_class_name_eval() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let code = "nil.class.to_s";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_STRING);
        let s = mrb_str_to_cstr(mrb, result) as *const i8;
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "NilClass");

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn define_module() {
    unsafe {
        let mrb = mrb_open();

        let mod_str = CString::new(TEST_MODULE).unwrap();
        let unknown_str = CString::new("UnknownModule").unwrap();

        mrb_define_module(mrb, mod_str.as_ptr());

        assert_eq!(mrb_class_defined(mrb, mod_str.as_ptr()), 1_u8);
        assert_eq!(mrb_class_defined(mrb, unknown_str.as_ptr()), 0_u8);

        mrb_close(mrb);
    }
}

#[test]
fn defined_under() {
    unsafe {
        let mrb = mrb_open();

        let kernel_str = CString::new("Kernel").unwrap();
        let kernel = mrb_module_get(mrb, kernel_str.as_ptr());
        let name_str = CString::new(TEST_MODULE).unwrap();
        let name = name_str.as_ptr();

        mrb_define_module_under(mrb, kernel, name);
        assert_eq!(mrb_class_defined_under(mrb, kernel, name), 1_u8);

        let mod_str = CString::new(TEST_MODULE).unwrap();
        let module = mrb_module_get(mrb, mod_str.as_ptr());
        let module_name = mrb_class_name(mrb, module);
        assert_eq!(
            CStr::from_ptr(module_name).to_str().unwrap(),
            "Kernel::TestModule"
        );

        mrb_close(mrb);
    }
}

#[test]
fn class_under() {
    unsafe {
        let mrb = mrb_open();

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let name_str = CString::new(TEST_CLASS).unwrap();
        let name = name_str.as_ptr();

        mrb_define_class_under(mrb, obj_class, name, obj_class);
        let new_class = mrb_class_get_under(mrb, obj_class, name);

        let name = mrb_class_name(mrb, new_class);

        assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "TestClass");

        mrb_close(mrb);
    }
}

#[test]
fn module_under() {
    unsafe {
        let mrb = mrb_open();

        let kernel_str = CString::new("Kernel").unwrap();
        let kernel = mrb_module_get(mrb, kernel_str.as_ptr());
        let name_str = CString::new(TEST_MODULE).unwrap();
        let name = name_str.as_ptr();

        mrb_define_module_under(mrb, kernel, name);
        let new_module = mrb_module_get_under(mrb, kernel, name);

        let name = mrb_class_name(mrb, new_module);

        assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "Kernel::TestModule");

        mrb_close(mrb);
    }
}

#[test]
fn include_module() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let code = "module Increment; def inc; self + 1; end; end";

        mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);

        let fixnum_str = CString::new("Fixnum").unwrap();
        let fixnum = mrb_class_get(mrb, fixnum_str.as_ptr());
        let increment_str = CString::new("Increment").unwrap();
        let increment = mrb_module_get(mrb, increment_str.as_ptr());

        mrb_include_module(mrb, fixnum, increment);

        let code = "1.inc";

        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn define_and_include_module() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let name_str = CString::new("Increment").unwrap();
        let name = name_str.as_ptr();
        mrb_define_module(mrb, name);
        let increment = mrb_module_get(mrb, name);

        extern "C" fn rust__mruby__increment__method__inc(
            _mrb: *mut mrb_state,
            slf: mrb_value,
        ) -> mrb_value {
            assert_eq!(slf.tt, mrb_vtype_MRB_TT_FIXNUM);
            // TODO write extension code to expose inline function
            // `mrb_fixnum_value`.
            //
            // `unsafe` block required because we're accessing a union field
            // which might access uninitialized memory. We know we this
            // operation is safe because of the above assert on `slf.tt`.
            unsafe {
                mrb_value {
                    value: mrb_value__bindgen_ty_1 { i: slf.value.i + 1 },
                    tt: mrb_vtype_MRB_TT_FIXNUM,
                }
            }
        }

        let inc_method_str = CString::new("inc").unwrap();

        mrb_define_method(
            mrb,
            increment,
            inc_method_str.as_ptr(),
            Some(rust__mruby__increment__method__inc),
            0,
        );

        let fixnum_str = CString::new("Fixnum").unwrap();
        let fixnum = mrb_class_get(mrb, fixnum_str.as_ptr());

        mrb_include_module(mrb, fixnum, increment);

        let code = "1.inc";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);

        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn define_class_method() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new(TEST_CLASS).unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        extern "C" fn rust__mruby__test_class__class_method__value(
            _mrb: *mut mrb_state,
            _slf: mrb_value,
        ) -> mrb_value {
            // TODO write extension code to expose inline function
            // `mrb_fixnum_value`.
            mrb_value {
                value: mrb_value__bindgen_ty_1 { i: 2 },
                tt: mrb_vtype_MRB_TT_FIXNUM,
            }
        }

        let new_method_str = CString::new("value").unwrap();

        mrb_define_class_method(
            mrb,
            new_class,
            new_method_str.as_ptr(),
            Some(rust__mruby__test_class__class_method__value),
            0,
        );

        let code = "TestClass.value";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn define_class_and_instance_method_with_one_rust_function() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new(TEST_CLASS).unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        extern "C" fn rust__mruby__test_class__method__value(
            _mrb: *mut mrb_state,
            slf: mrb_value,
        ) -> mrb_value {
            // TODO write extension code to expose inline function
            // `mrb_fixnum_value`.
            match slf.tt {
                mrb_vtype_MRB_TT_OBJECT => mrb_value {
                    value: mrb_value__bindgen_ty_1 { i: 2 },
                    tt: mrb_vtype_MRB_TT_FIXNUM,
                },
                mrb_vtype_MRB_TT_CLASS => mrb_value {
                    value: mrb_value__bindgen_ty_1 { i: 3 },
                    tt: mrb_vtype_MRB_TT_FIXNUM,
                },
                tt => unreachable!("unexpected mrb_value type: {}", tt),
            }
        }
        let rust__mruby__test_class__class_method__value = rust__mruby__test_class__method__value;

        let new_method_str = CString::new("value").unwrap();
        let new_class_method_str = CString::new("value").unwrap();

        mrb_define_method(
            mrb,
            new_class,
            new_method_str.as_ptr(),
            Some(rust__mruby__test_class__method__value),
            0,
        );
        mrb_define_class_method(
            mrb,
            new_class,
            new_class_method_str.as_ptr(),
            Some(rust__mruby__test_class__class_method__value),
            0,
        );

        let code = "TestClass.value + TestClass.new.value";
        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 5);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn define_constant() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let kernel_str = CString::new("Kernel").unwrap();
        let kernel = mrb_module_get(mrb, kernel_str.as_ptr());

        let one = mrb_value {
            value: mrb_value__bindgen_ty_1 { i: 1 },
            tt: mrb_vtype_MRB_TT_FIXNUM,
        };
        let one_str = CString::new("ONE").unwrap();

        // Define constant on Class
        mrb_define_const(mrb, obj_class, one_str.as_ptr(), one);
        // Define constant on Module
        mrb_define_const(mrb, kernel, one_str.as_ptr(), one);

        let code = "Object::ONE";

        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 1);

        let code = "Kernel::ONE";

        let result = mrb_load_nstring_cxt(mrb, code.as_ptr() as *const i8, code.len(), context);
        assert_eq!(result.tt, mrb_vtype_MRB_TT_FIXNUM);
        assert_eq!(result.value.i, 1);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn protect() {
    use std::mem::uninitialized;

    unsafe {
        let mrb = mrb_open();

        // This rust function raises a runtime error in mruby
        extern "C" fn rust__mruby__test_class__method__value(
            mrb: *mut mrb_state,
            _data: mrb_value,
        ) -> mrb_value {
            unsafe {
                let eclass = CString::new("RuntimeError").unwrap();
                let msg = CString::new("excepting").unwrap();
                mrb_raise(
                    mrb as *mut mrb_state,
                    mrb_class_get(mrb as *mut mrb_state, eclass.as_ptr()),
                    msg.as_ptr(),
                );

                mrb_value {
                    tt: mrb_vtype_MRB_TT_FIXNUM,
                    value: mrb_value__bindgen_ty_1 { i: 7 },
                }
            }
        }

        let mut state = uninitialized::<u8>();
        let nil = mrb_value {
            tt: mrb_vtype_MRB_TT_FALSE,
            value: mrb_value__bindgen_ty_1 { i: 0 },
        };

        // `mrb_protect` calls the passed function with `data` as an argument.
        // Protect wraps the execution of the provided function in a try catch.
        // If the passed function raises, the `state` out variable is set to
        // true and the returned `mrb_value` will be an exception. If no
        // exception is thrown, the returned `mrb_value` is the value returned
        // by the provided function.
        //
        // The function we are passing below raises a `RuntimeException`, so we
        // expect `exc` to be an `Exception`.
        let exc = mrb_protect(
            mrb as *mut mrb_state,
            Some(rust__mruby__test_class__method__value),
            nil,
            &mut state,
        );

        // state == true means an exception was thrown by the protected function
        assert_eq!(state, 1_u8);
        assert_eq!(exc.tt, mrb_vtype_MRB_TT_EXCEPTION);

        let args = &[];

        // This code calls `.class.to_s` on the `RuntimeException` our
        // protected `value` method raised.
        let class_str = "class";
        let class_sym = mrb_intern(mrb, class_str.as_ptr() as *const i8, class_str.len());
        let to_s_str = "to_s";
        let to_s_sym = mrb_intern(mrb, to_s_str.as_ptr() as *const i8, to_s_str.len());

        // `mrb_funcall_argv` calls a method named by a symbol with a list of
        // arguments on an `mrb_value`.
        let class = mrb_funcall_argv(mrb, exc, class_sym, 0, args.as_ptr());
        let result = mrb_funcall_argv(mrb, class, to_s_sym, 0, args.as_ptr());

        assert_eq!(result.tt, mrb_vtype_MRB_TT_STRING);
        let s = mrb_str_to_cstr(mrb, result) as *const i8;
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "RuntimeError");

        mrb_close(mrb);
    }
}

/*
#[test]
pub fn args() {
    use std::mem::uninitialized;

    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        extern "C" fn add(mrb: *const MrState, _slf: MrValue) -> MrValue {
            unsafe {
                let a = uninitialized::<MrValue>();
                let b = uninitialized::<MrValue>();

                let sig_str = CString::new("oo").unwrap();

                mrb_get_args(mrb, sig_str.as_ptr(), &a as *const MrValue,
                             &b as *const MrValue);

                let args = &[b];

                let plus_str = CString::new("+").unwrap();

                let sym = mrb_intern(mrb, plus_str.as_ptr(), 1usize);

                mrb_funcall_argv(mrb, a, sym, 1, args.as_ptr())
            }
        }

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new("Mine").unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        let add_str = CString::new("add").unwrap();

        mrb_define_method(mrb, new_class, add_str.as_ptr(), add,
                          (2 & 0x1f) << 18);

        let code = "Mine.new.add 1, 1";

        assert_eq!(mrb_load_nstring_cxt(mrb, code.as_ptr(), code.len() as i32, context)
                   .to_i32().unwrap(), 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
pub fn str_args() {
    use std::ffi::CStr;
    use std::mem::uninitialized;
    use std::os::raw::c_char;

    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        extern "C" fn add(mrb: *const MrState, _slf: MrValue) -> MrValue {
            unsafe {
                let a = uninitialized::<*const c_char>();
                let b = uninitialized::<*const c_char>();

                let sig_str = CString::new("zz").unwrap();

                mrb_get_args(mrb, sig_str.as_ptr(), &a as *const *const c_char,
                             &b as *const *const c_char);

                let a = CStr::from_ptr(a).to_str().unwrap();
                let b = CStr::from_ptr(b).to_str().unwrap();

                let args = &[MrValue::string(mrb, b)];

                let plus_str = CString::new("+").unwrap();

                let sym = mrb_intern(mrb, plus_str.as_ptr(), 1usize);

                mrb_funcall_argv(mrb, MrValue::string(mrb, a), sym, 1, args.as_ptr())
            }
        }

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new("Mine").unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        let add_str = CString::new("add").unwrap();

        mrb_define_method(mrb, new_class, add_str.as_ptr(), add,
                          (2 & 0x1f) << 18);

        let code = "Mine.new.add 'a', 'b'";

        assert_eq!(mrb_load_nstring_cxt(mrb, code.as_ptr(), code.len() as i32, context)
                   .to_str(mrb).unwrap(), "ab");

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
pub fn array_args() {
    use std::mem::uninitialized;

    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        extern "C" fn add(mrb: *const MrState, _slf: MrValue) -> MrValue {
            unsafe {
                let array = uninitialized::<MrValue>();

                let a_str = CString::new("A").unwrap();

                mrb_get_args(mrb, a_str.as_ptr(), &array as *const MrValue);

                let vec = array.to_vec(mrb).unwrap();

                let args = &[vec[1]];

                let plus_str = CString::new("+").unwrap();

                let sym = mrb_intern(mrb, plus_str.as_ptr(), 1usize);

                mrb_funcall_argv(mrb, vec[0], sym, 1, args.as_ptr())
            }
        }

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_class_str = CString::new("Mine").unwrap();
        let new_class = mrb_define_class(mrb, new_class_str.as_ptr(), obj_class);

        let add_str = CString::new("add").unwrap();

        mrb_define_method(mrb, new_class, add_str.as_ptr(), add,
                          (2 & 0x1f) << 18);

        let code = "Mine.new.add [1, 1]";

        assert_eq!(mrb_load_nstring_cxt(mrb, code.as_ptr(), code.len() as i32, context)
                   .to_i32().unwrap(), 2);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}

#[test]
fn funcall_argv() {
    unsafe {
        let mrb = mrb_open();

        let one = MrValue::fixnum(1);
        let args = &[MrValue::fixnum(2)];

        let plus_str = CString::new("+").unwrap();

        let sym = mrb_intern(mrb, plus_str.as_ptr(), 1usize);

        let result = mrb_funcall_argv(mrb, one, sym, 1, args.as_ptr());

        assert_eq!(result.to_i32().unwrap(), 3);

        mrb_close(mrb);
    }
}

#[test]
fn iv() {
    unsafe {
        let mrb = mrb_open();
        let context = mrbc_context_new(mrb);

        let obj_str = CString::new("Object").unwrap();
        let obj_class = mrb_class_get(mrb, obj_str.as_ptr());
        let new_str = CString::new("Mine").unwrap();

        mrb_define_class(mrb, new_str.as_ptr(), obj_class);

        let one = MrValue::fixnum(1);

        let code = "Mine.new";
        let obj = mrb_load_nstring_cxt(mrb, code.as_ptr(), code.len() as i32, context);

        let value_str = CString::new("value").unwrap();

        let sym = mrb_intern(mrb, value_str.as_ptr(), 1usize);

        assert!(!mrb_iv_defined(mrb, obj, sym));

        mrb_iv_set(mrb, obj, sym, one);

        assert!(mrb_iv_defined(mrb, obj, sym));
        assert_eq!(mrb_iv_get(mrb, obj, sym).to_i32().unwrap(), 1);

        mrbc_context_free(mrb, context);
        mrb_close(mrb);
    }
}
*/
