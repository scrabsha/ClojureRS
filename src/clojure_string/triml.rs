use crate::ifn::IFn;
use crate::value::{ToValue, Value};
use std::rc::Rc;

use crate::error_message;
use crate::type_tag::TypeTag;

/// clojure.string/triml trims white space from start of string
#[derive(Debug, Clone)]
pub struct TrimLFn {}
impl ToValue for TrimLFn {
    fn to_value(&self) -> Value {
        Value::IFn(Rc::new(self.clone()))
    }
}
impl IFn for TrimLFn {
    fn invoke(&self, args: Vec<Rc<Value>>) -> Value {
        if args.len() != 1 {
            return error_message::wrong_arg_count(1, args.len());
        } else {
            match args.get(0).unwrap().to_value() {
                Value::String(s) => Value::String(s.trim_start().to_string()),
                _a => error_message::type_mismatch(TypeTag::String, &_a.to_value()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    mod triml_tests {
        use crate::clojure_string::triml::TrimLFn;
        use crate::ifn::IFn;
        use crate::value::Value;
        use std::rc::Rc;

        #[test]
        fn triml() {
            let triml = TrimLFn {};
            let s = " \r \t  hello   \n";
            let args = vec![Rc::new(Value::String(String::from(s)))];
            assert_eq!(
                Value::String(String::from("hello   \n")),
                triml.invoke(args)
            );
        }
    }
}
