use crate::convert::{Convert, Error, TryConvert};
use crate::types::{Ruby, Rust};
use crate::value::Value;
use crate::Artichoke;

impl Convert<Vec<bool>> for Value {
    type From = Rust;
    type To = Ruby;

    fn convert(interp: &Artichoke, value: Vec<bool>) -> Self {
        let mut values = Vec::with_capacity(value.len());
        for item in value {
            values.push(Self::convert(interp, item));
        }
        Self::convert(interp, values)
    }
}

#[allow(clippy::use_self)]
// https://github.com/rust-lang/rust-clippy/issues/4143
impl TryConvert<Value> for Vec<bool> {
    type From = Ruby;
    type To = Rust;

    unsafe fn try_convert(
        interp: &Artichoke,
        value: Value,
    ) -> Result<Self, Error<Self::From, Self::To>> {
        let values = <Vec<Value>>::try_convert(interp, value)?;
        let mut vec = Self::with_capacity(values.len());
        for item in values {
            vec.push(bool::try_convert(interp, item)?);
        }
        Ok(vec)
    }
}

impl Convert<Vec<Option<bool>>> for Value {
    type From = Rust;
    type To = Ruby;

    fn convert(interp: &Artichoke, value: Vec<Option<bool>>) -> Self {
        let mut values = Vec::with_capacity(value.len());
        for item in value {
            values.push(Self::convert(interp, item));
        }
        Self::convert(interp, values)
    }
}

#[allow(clippy::use_self)]
// https://github.com/rust-lang/rust-clippy/issues/4143
impl TryConvert<Value> for Vec<Option<bool>> {
    type From = Ruby;
    type To = Rust;

    unsafe fn try_convert(
        interp: &Artichoke,
        value: Value,
    ) -> Result<Self, Error<Self::From, Self::To>> {
        let values = <Vec<Value>>::try_convert(interp, value)?;
        let mut vec = Self::with_capacity(values.len());
        for item in values {
            vec.push(<Option<bool>>::try_convert(interp, item)?);
        }
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck_macros::quickcheck;
    use std::convert::TryFrom;

    use crate::convert::{Convert, Error, TryConvert};
    use crate::eval::Eval;
    use crate::sys;
    use crate::types::{Ruby, Rust};
    use crate::value::Value;

    #[test]
    fn fail_convert() {
        let interp = crate::interpreter().expect("init");
        // get a mrb_value that can't be converted to a primitive type.
        let value = interp.eval("Object.new").expect("eval");
        let expected = Error {
            from: Ruby::Object,
            to: Rust::Vec,
        };
        let result = unsafe { <Vec<bool>>::try_convert(&interp, value) }.map(|_| ());
        assert_eq!(result, Err(expected));
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn convert_to_value(v: Vec<bool>) -> bool {
        let interp = crate::interpreter().expect("init");
        let value = Value::convert(&interp, v.clone());
        let inner = value.inner();
        let size = i64::try_from(v.len()).expect("vec size");
        unsafe { sys::mrb_sys_ary_len(inner) == size }
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn roundtrip(v: Vec<bool>) -> bool {
        let interp = crate::interpreter().expect("init");
        let value = Value::convert(&interp, v.clone());
        unsafe { <Vec<bool>>::try_convert(&interp, value) == Ok(v) }
    }
}
