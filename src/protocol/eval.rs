use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::{i16, i32, i64, i8};

use crate::common::*;

use crate::protocol::ast::*;
use crate::protocol::inputsource::*;
use crate::protocol::parser::*;
use crate::protocol::EvalContext;

const MAX_RECURSION: usize = 1024;

const BYTE_MIN: i64 = i8::MIN as i64;
const BYTE_MAX: i64 = i8::MAX as i64;
const SHORT_MIN: i64 = i16::MIN as i64;
const SHORT_MAX: i64 = i16::MAX as i64;
const INT_MIN: i64 = i32::MIN as i64;
const INT_MAX: i64 = i32::MAX as i64;

const MESSAGE_MAX_LENGTH: i64 = SHORT_MAX;

const ONE: Value = Value::Byte(ByteValue(1));

trait ValueImpl {
    fn exact_type(&self) -> Type;
    fn is_type_compatible(&self, t: &Type) -> bool;
}

#[derive(Debug, Clone)]
pub enum Value {
    Input(InputValue),
    Output(OutputValue),
    Message(MessageValue),
    Boolean(BooleanValue),
    Byte(ByteValue),
    Short(ShortValue),
    Int(IntValue),
    Long(LongValue),
    InputArray(InputArrayValue),
    OutputArray(OutputArrayValue),
    MessageArray(MessageArrayValue),
    BooleanArray(BooleanArrayValue),
    ByteArray(ByteArrayValue),
    ShortArray(ShortArrayValue),
    IntArray(IntArrayValue),
    LongArray(LongArrayValue),
}
impl Value {
    pub fn receive_message(buffer: &Vec<u8>) -> Value {
        Value::Message(MessageValue(Some(buffer.clone())))
    }
    fn create_message(length: Value) -> Value {
        match length {
            Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) => {
                let length: i64 = i64::from(length);
                if length < 0 || length > MESSAGE_MAX_LENGTH {
                    // Only messages within the expected length are allowed
                    Value::Message(MessageValue(None))
                } else {
                    Value::Message(MessageValue(Some(vec![0; length.try_into().unwrap()])))
                }
            }
            _ => unimplemented!(),
        }
    }
    fn from_constant(constant: &Constant) -> Value {
        match constant {
            Constant::Null => Value::Message(MessageValue(None)),
            Constant::True => Value::Boolean(BooleanValue(true)),
            Constant::False => Value::Boolean(BooleanValue(false)),
            Constant::Integer(data) => {
                // Convert raw ASCII data to UTF-8 string
                let raw = String::from_utf8_lossy(data);
                let val = raw.parse::<i64>().unwrap();
                if val >= BYTE_MIN && val <= BYTE_MAX {
                    Value::Byte(ByteValue(val as i8))
                } else if val >= SHORT_MIN && val <= SHORT_MAX {
                    Value::Short(ShortValue(val as i16))
                } else if val >= INT_MIN && val <= INT_MAX {
                    Value::Int(IntValue(val as i32))
                } else {
                    Value::Long(LongValue(val))
                }
            }
            Constant::Character(data) => unimplemented!(),
        }
    }
    fn set(&mut self, index: &Value, value: &Value) -> Option<Value> {
        // The index must be of integer type, and non-negative
        let the_index: usize;
        match index {
            Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) => {
                let index = i64::from(index);
                if index < 0 || index >= MESSAGE_MAX_LENGTH {
                    // It is inconsistent to update out of bounds
                    return None;
                }
                the_index = index.try_into().unwrap();
            }
            _ => unreachable!(),
        }
        // The subject must be either a message or an array
        // And the value and the subject must be compatible
        match (self, value) {
            (Value::Message(MessageValue(None)), _) => {
                // It is inconsistent to update the null message
                None
            }
            (Value::Message(MessageValue(Some(buffer))), Value::Byte(ByteValue(b))) => {
                if *b < 0 {
                    // It is inconsistent to update with a negative value
                    return None;
                }
                if let Some(slot) = buffer.get_mut(the_index) {
                    *slot = (*b).try_into().unwrap();
                    Some(value.clone())
                } else {
                    // It is inconsistent to update out of bounds
                    None
                }
            }
            (Value::Message(MessageValue(Some(buffer))), Value::Short(ShortValue(b))) => {
                if *b < 0 || *b > BYTE_MAX as i16 {
                    // It is inconsistent to update with a negative value or a too large value
                    return None;
                }
                if let Some(slot) = buffer.get_mut(the_index) {
                    *slot = (*b).try_into().unwrap();
                    Some(value.clone())
                } else {
                    // It is inconsistent to update out of bounds
                    None
                }
            }
            (Value::InputArray(_), Value::Input(_)) => todo!(),
            (Value::OutputArray(_), Value::Output(_)) => todo!(),
            (Value::MessageArray(_), Value::Message(_)) => todo!(),
            (Value::BooleanArray(_), Value::Boolean(_)) => todo!(),
            (Value::ByteArray(_), Value::Byte(_)) => todo!(),
            (Value::ShortArray(_), Value::Short(_)) => todo!(),
            (Value::IntArray(_), Value::Int(_)) => todo!(),
            (Value::LongArray(_), Value::Long(_)) => todo!(),
            _ => unreachable!(),
        }
    }
    fn get(&self, index: &Value) -> Option<Value> {
        // The index must be of integer type, and non-negative
        let the_index: usize;
        match index {
            Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) => {
                let index = i64::from(index);
                if index < 0 || index >= MESSAGE_MAX_LENGTH {
                    // It is inconsistent to update out of bounds
                    return None;
                }
                the_index = index.try_into().unwrap();
            }
            _ => unreachable!(),
        }
        // The subject must be either a message or an array
        match self {
            Value::Message(MessageValue(None)) => {
                // It is inconsistent to read from the null message
                None
            }
            Value::Message(MessageValue(Some(buffer))) => {
                if let Some(slot) = buffer.get(the_index) {
                    Some(Value::Short(ShortValue((*slot).try_into().unwrap())))
                } else {
                    // It is inconsistent to update out of bounds
                    None
                }
            }
            Value::InputArray(_) => todo!(),
            Value::OutputArray(_) => todo!(),
            Value::MessageArray(_) => todo!(),
            Value::BooleanArray(_) => todo!(),
            Value::ByteArray(_) => todo!(),
            Value::ShortArray(_) => todo!(),
            Value::IntArray(_) => todo!(),
            Value::LongArray(_) => todo!(),
            _ => unreachable!(),
        }
    }
    fn length(&self) -> Option<Value> {
        // The subject must be either a message or an array
        match self {
            Value::Message(MessageValue(None)) => {
                // It is inconsistent to get length from the null message
                None
            }
            Value::Message(MessageValue(Some(buffer))) => {
                Some(Value::Int(IntValue((buffer.len()).try_into().unwrap())))
            }
            Value::InputArray(InputArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::OutputArray(OutputArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::MessageArray(MessageArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::BooleanArray(BooleanArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::ByteArray(ByteArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::ShortArray(ShortArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::IntArray(IntArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            Value::LongArray(LongArrayValue(vec)) => {
                Some(Value::Int(IntValue((vec.len()).try_into().unwrap())))
            }
            _ => unreachable!(),
        }
    }
    fn plus(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Byte(ByteValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Byte(ByteValue(*s + *o))
            }
            (Value::Byte(ByteValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s as i16 + *o))
            }
            (Value::Byte(ByteValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 + *o))
            }
            (Value::Byte(ByteValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 + *o))
            }
            (Value::Short(ShortValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Short(ShortValue(*s + *o as i16))
            }
            (Value::Short(ShortValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s + *o))
            }
            (Value::Short(ShortValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 + *o))
            }
            (Value::Short(ShortValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 + *o))
            }
            (Value::Int(IntValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Int(IntValue(*s + *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Short(ShortValue(o))) => {
                Value::Int(IntValue(*s + *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Int(IntValue(o))) => Value::Int(IntValue(*s + *o)),
            (Value::Int(IntValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 + *o))
            }
            (Value::Long(LongValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Long(LongValue(*s + *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Short(ShortValue(o))) => {
                Value::Long(LongValue(*s + *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Int(IntValue(o))) => {
                Value::Long(LongValue(*s + *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s + *o))
            }
            _ => unimplemented!(),
        }
    }
    fn minus(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Byte(ByteValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Byte(ByteValue(*s - *o))
            }
            (Value::Byte(ByteValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s as i16 - *o))
            }
            (Value::Byte(ByteValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 - *o))
            }
            (Value::Byte(ByteValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 - *o))
            }
            (Value::Short(ShortValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Short(ShortValue(*s - *o as i16))
            }
            (Value::Short(ShortValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s - *o))
            }
            (Value::Short(ShortValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 - *o))
            }
            (Value::Short(ShortValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 - *o))
            }
            (Value::Int(IntValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Int(IntValue(*s - *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Short(ShortValue(o))) => {
                Value::Int(IntValue(*s - *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Int(IntValue(o))) => Value::Int(IntValue(*s - *o)),
            (Value::Int(IntValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 - *o))
            }
            (Value::Long(LongValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Long(LongValue(*s - *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Short(ShortValue(o))) => {
                Value::Long(LongValue(*s - *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Int(IntValue(o))) => {
                Value::Long(LongValue(*s - *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s - *o))
            }
            _ => unimplemented!(),
        }
    }
    fn modulus(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Byte(ByteValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Byte(ByteValue(*s % *o))
            }
            (Value::Byte(ByteValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s as i16 % *o))
            }
            (Value::Byte(ByteValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 % *o))
            }
            (Value::Byte(ByteValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 % *o))
            }
            (Value::Short(ShortValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Short(ShortValue(*s % *o as i16))
            }
            (Value::Short(ShortValue(s)), Value::Short(ShortValue(o))) => {
                Value::Short(ShortValue(*s % *o))
            }
            (Value::Short(ShortValue(s)), Value::Int(IntValue(o))) => {
                Value::Int(IntValue(*s as i32 % *o))
            }
            (Value::Short(ShortValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 % *o))
            }
            (Value::Int(IntValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Int(IntValue(*s % *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Short(ShortValue(o))) => {
                Value::Int(IntValue(*s % *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Int(IntValue(o))) => Value::Int(IntValue(*s % *o)),
            (Value::Int(IntValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s as i64 % *o))
            }
            (Value::Long(LongValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Long(LongValue(*s % *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Short(ShortValue(o))) => {
                Value::Long(LongValue(*s % *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Int(IntValue(o))) => {
                Value::Long(LongValue(*s % *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Long(LongValue(o))) => {
                Value::Long(LongValue(*s % *o))
            }
            _ => unimplemented!(),
        }
    }
    fn eq(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Byte(ByteValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o))
            }
            (Value::Byte(ByteValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s as i16 == *o))
            }
            (Value::Byte(ByteValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s as i32 == *o))
            }
            (Value::Byte(ByteValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 == *o))
            }
            (Value::Short(ShortValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i16))
            }
            (Value::Short(ShortValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o))
            }
            (Value::Short(ShortValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s as i32 == *o))
            }
            (Value::Short(ShortValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 == *o))
            }
            (Value::Int(IntValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o))
            }
            (Value::Int(IntValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 == *o))
            }
            (Value::Long(LongValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o))
            }
            (Value::Message(MessageValue(s)), Value::Message(MessageValue(o))) => {
                Value::Boolean(BooleanValue(*s == *o))
            }
            _ => unimplemented!(),
        }
    }
    fn neq(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Byte(ByteValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o))
            }
            (Value::Byte(ByteValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s as i16 != *o))
            }
            (Value::Byte(ByteValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s as i32 != *o))
            }
            (Value::Byte(ByteValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 != *o))
            }
            (Value::Short(ShortValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i16))
            }
            (Value::Short(ShortValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o))
            }
            (Value::Short(ShortValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s as i32 != *o))
            }
            (Value::Short(ShortValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 != *o))
            }
            (Value::Int(IntValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i32))
            }
            (Value::Int(IntValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o))
            }
            (Value::Int(IntValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s as i64 != *o))
            }
            (Value::Long(LongValue(s)), Value::Byte(ByteValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Short(ShortValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Int(IntValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o as i64))
            }
            (Value::Long(LongValue(s)), Value::Long(LongValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o))
            }
            (Value::Message(MessageValue(s)), Value::Message(MessageValue(o))) => {
                Value::Boolean(BooleanValue(*s != *o))
            }
            _ => unimplemented!(),
        }
    }
    fn lt(&self, other: &Value) -> Value {
        // TODO: match value directly (as done above)
        assert!(!self.exact_type().array);
        assert!(!other.exact_type().array);
        match (self.exact_type().primitive, other.exact_type().primitive) {
            (PrimitiveType::Byte, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i8::from(self) < i8::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) < i16::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) < i32::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i16::from(self) < i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) < i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) < i32::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i32::from(self) < i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i32::from(self) < i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) < i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) < i64::from(other)))
            }
            _ => unimplemented!(),
        }
    }
    fn lte(&self, other: &Value) -> Value {
        assert!(!self.exact_type().array);
        assert!(!other.exact_type().array);
        match (self.exact_type().primitive, other.exact_type().primitive) {
            (PrimitiveType::Byte, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i8::from(self) <= i8::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) <= i16::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) <= i32::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i16::from(self) <= i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) <= i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) <= i32::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i32::from(self) <= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i32::from(self) <= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) <= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) <= i64::from(other)))
            }
            _ => unimplemented!(),
        }
    }
    fn gt(&self, other: &Value) -> Value {
        assert!(!self.exact_type().array);
        assert!(!other.exact_type().array);
        match (self.exact_type().primitive, other.exact_type().primitive) {
            (PrimitiveType::Byte, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i8::from(self) > i8::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) > i16::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) > i32::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i16::from(self) > i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) > i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) > i32::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i32::from(self) > i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i32::from(self) > i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) > i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) > i64::from(other)))
            }
            _ => unimplemented!(),
        }
    }
    fn gte(&self, other: &Value) -> Value {
        assert!(!self.exact_type().array);
        assert!(!other.exact_type().array);
        match (self.exact_type().primitive, other.exact_type().primitive) {
            (PrimitiveType::Byte, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i8::from(self) >= i8::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) >= i16::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) >= i32::from(other)))
            }
            (PrimitiveType::Byte, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i16::from(self) >= i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i16::from(self) >= i16::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) >= i32::from(other)))
            }
            (PrimitiveType::Short, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i32::from(self) >= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i32::from(self) >= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i32::from(self) >= i32::from(other)))
            }
            (PrimitiveType::Int, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Byte) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Short) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Int) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            (PrimitiveType::Long, PrimitiveType::Long) => {
                Value::Boolean(BooleanValue(i64::from(self) >= i64::from(other)))
            }
            _ => unimplemented!(),
        }
    }
    fn as_boolean(&self) -> &BooleanValue {
        match self {
            Value::Boolean(result) => result,
            _ => panic!("Unable to cast `Value` to `BooleanValue`"),
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(BooleanValue(b))
    }
}
impl From<Value> for bool {
    fn from(val: Value) -> Self {
        match val {
            Value::Boolean(BooleanValue(b)) => b,
            _ => unimplemented!(),
        }
    }
}
impl From<&Value> for bool {
    fn from(val: &Value) -> Self {
        match val {
            Value::Boolean(BooleanValue(b)) => *b,
            _ => unimplemented!(),
        }
    }
}

impl From<Value> for i8 {
    fn from(val: Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => b,
            _ => unimplemented!(),
        }
    }
}
impl From<&Value> for i8 {
    fn from(val: &Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => *b,
            _ => unimplemented!(),
        }
    }
}

impl From<Value> for i16 {
    fn from(val: Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i16::from(b),
            Value::Short(ShortValue(s)) => s,
            _ => unimplemented!(),
        }
    }
}
impl From<&Value> for i16 {
    fn from(val: &Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i16::from(*b),
            Value::Short(ShortValue(s)) => *s,
            _ => unimplemented!(),
        }
    }
}

impl From<Value> for i32 {
    fn from(val: Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i32::from(b),
            Value::Short(ShortValue(s)) => i32::from(s),
            Value::Int(IntValue(i)) => i,
            _ => unimplemented!(),
        }
    }
}
impl From<&Value> for i32 {
    fn from(val: &Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i32::from(*b),
            Value::Short(ShortValue(s)) => i32::from(*s),
            Value::Int(IntValue(i)) => *i,
            _ => unimplemented!(),
        }
    }
}

impl From<Value> for i64 {
    fn from(val: Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i64::from(b),
            Value::Short(ShortValue(s)) => i64::from(s),
            Value::Int(IntValue(i)) => i64::from(i),
            Value::Long(LongValue(l)) => l,
            _ => unimplemented!(),
        }
    }
}
impl From<&Value> for i64 {
    fn from(val: &Value) -> Self {
        match val {
            Value::Byte(ByteValue(b)) => i64::from(*b),
            Value::Short(ShortValue(s)) => i64::from(*s),
            Value::Int(IntValue(i)) => i64::from(*i),
            Value::Long(LongValue(l)) => *l,
            _ => unimplemented!(),
        }
    }
}

impl ValueImpl for Value {
    fn exact_type(&self) -> Type {
        match self {
            Value::Input(val) => val.exact_type(),
            Value::Output(val) => val.exact_type(),
            Value::Message(val) => val.exact_type(),
            Value::Boolean(val) => val.exact_type(),
            Value::Byte(val) => val.exact_type(),
            Value::Short(val) => val.exact_type(),
            Value::Int(val) => val.exact_type(),
            Value::Long(val) => val.exact_type(),
            Value::InputArray(val) => val.exact_type(),
            Value::OutputArray(val) => val.exact_type(),
            Value::MessageArray(val) => val.exact_type(),
            Value::BooleanArray(val) => val.exact_type(),
            Value::ByteArray(val) => val.exact_type(),
            Value::ShortArray(val) => val.exact_type(),
            Value::IntArray(val) => val.exact_type(),
            Value::LongArray(val) => val.exact_type(),
        }
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        match self {
            Value::Input(val) => val.is_type_compatible(t),
            Value::Output(val) => val.is_type_compatible(t),
            Value::Message(val) => val.is_type_compatible(t),
            Value::Boolean(val) => val.is_type_compatible(t),
            Value::Byte(val) => val.is_type_compatible(t),
            Value::Short(val) => val.is_type_compatible(t),
            Value::Int(val) => val.is_type_compatible(t),
            Value::Long(val) => val.is_type_compatible(t),
            Value::InputArray(val) => val.is_type_compatible(t),
            Value::OutputArray(val) => val.is_type_compatible(t),
            Value::MessageArray(val) => val.is_type_compatible(t),
            Value::BooleanArray(val) => val.is_type_compatible(t),
            Value::ByteArray(val) => val.is_type_compatible(t),
            Value::ShortArray(val) => val.is_type_compatible(t),
            Value::IntArray(val) => val.is_type_compatible(t),
            Value::LongArray(val) => val.is_type_compatible(t),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let disp: &dyn Display;
        match self {
            Value::Input(val) => disp = val,
            Value::Output(val) => disp = val,
            Value::Message(val) => disp = val,
            Value::Boolean(val) => disp = val,
            Value::Byte(val) => disp = val,
            Value::Short(val) => disp = val,
            Value::Int(val) => disp = val,
            Value::Long(val) => disp = val,
            Value::InputArray(val) => disp = val,
            Value::OutputArray(val) => disp = val,
            Value::MessageArray(val) => disp = val,
            Value::BooleanArray(val) => disp = val,
            Value::ByteArray(val) => disp = val,
            Value::ShortArray(val) => disp = val,
            Value::IntArray(val) => disp = val,
            Value::LongArray(val) => disp = val,
        }
        disp.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct InputValue(pub Key);

impl Display for InputValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#in")
    }
}

impl ValueImpl for InputValue {
    fn exact_type(&self) -> Type {
        Type::INPUT
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Input => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputValue(pub Key);

impl Display for OutputValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#out")
    }
}

impl ValueImpl for OutputValue {
    fn exact_type(&self) -> Type {
        Type::OUTPUT
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Output => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageValue(pub Option<Vec<u8>>);

impl Display for MessageValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            None => write!(f, "null"),
            Some(vec) => {
                write!(f, "#msg({};", vec.len())?;
                let mut i = 0;
                for v in vec.iter() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", v)?;
                    i += 1;
                    if i >= 10 {
                        write!(f, ",...")?;
                        break;
                    }
                }
                write!(f, ")")
            }
        }
    }
}

impl ValueImpl for MessageValue {
    fn exact_type(&self) -> Type {
        Type::MESSAGE
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Message => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BooleanValue(bool);

impl Display for BooleanValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ValueImpl for BooleanValue {
    fn exact_type(&self) -> Type {
        Type::BOOLEAN
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Boolean => true,
            PrimitiveType::Byte => true,
            PrimitiveType::Short => true,
            PrimitiveType::Int => true,
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ByteValue(i8);

impl Display for ByteValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ValueImpl for ByteValue {
    fn exact_type(&self) -> Type {
        Type::BYTE
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Byte => true,
            PrimitiveType::Short => true,
            PrimitiveType::Int => true,
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShortValue(i16);

impl Display for ShortValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ValueImpl for ShortValue {
    fn exact_type(&self) -> Type {
        Type::SHORT
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Short => true,
            PrimitiveType::Int => true,
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntValue(i32);

impl Display for IntValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ValueImpl for IntValue {
    fn exact_type(&self) -> Type {
        Type::INT
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Int => true,
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LongValue(i64);

impl Display for LongValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ValueImpl for LongValue {
    fn exact_type(&self) -> Type {
        Type::LONG
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if *array {
            return false;
        }
        match primitive {
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputArrayValue(Vec<InputValue>);

impl Display for InputArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for InputArrayValue {
    fn exact_type(&self) -> Type {
        Type::INPUT_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Input => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputArrayValue(Vec<OutputValue>);

impl Display for OutputArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for OutputArrayValue {
    fn exact_type(&self) -> Type {
        Type::OUTPUT_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Output => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageArrayValue(Vec<MessageValue>);

impl Display for MessageArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for MessageArrayValue {
    fn exact_type(&self) -> Type {
        Type::MESSAGE_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Message => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BooleanArrayValue(Vec<BooleanValue>);

impl Display for BooleanArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for BooleanArrayValue {
    fn exact_type(&self) -> Type {
        Type::BOOLEAN_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Boolean => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ByteArrayValue(Vec<ByteValue>);

impl Display for ByteArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for ByteArrayValue {
    fn exact_type(&self) -> Type {
        Type::BYTE_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Byte => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShortArrayValue(Vec<ShortValue>);

impl Display for ShortArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for ShortArrayValue {
    fn exact_type(&self) -> Type {
        Type::SHORT_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Short => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntArrayValue(Vec<IntValue>);

impl Display for IntArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for IntArrayValue {
    fn exact_type(&self) -> Type {
        Type::INT_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Int => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LongArrayValue(Vec<LongValue>);

impl Display for LongArrayValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl ValueImpl for LongArrayValue {
    fn exact_type(&self) -> Type {
        Type::LONG_ARRAY
    }
    fn is_type_compatible(&self, t: &Type) -> bool {
        let Type { primitive, array } = t;
        if !*array {
            return false;
        }
        match primitive {
            PrimitiveType::Long => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct Store {
    map: HashMap<VariableId, Value>,
}
impl Store {
    fn new() -> Self {
        Store { map: HashMap::new() }
    }
    fn initialize(&mut self, h: &Heap, var: VariableId, value: Value) {
        // Ensure value is compatible with type of variable
        let the_type = h[var].the_type(h);
        assert!(value.is_type_compatible(the_type));
        // Overwrite mapping
        self.map.insert(var, value.clone());
    }
    fn update(
        &mut self,
        h: &Heap,
        ctx: &mut EvalContext,
        lexpr: ExpressionId,
        value: Value,
    ) -> EvalResult {
        match &h[lexpr] {
            Expression::Variable(var) => {
                let var = var.declaration.unwrap();
                // Ensure value is compatible with type of variable
                let the_type = h[var].the_type(h);
                assert!(value.is_type_compatible(the_type));
                // Overwrite mapping
                self.map.insert(var, value.clone());
                Ok(value)
            }
            Expression::Indexing(indexing) => {
                // Evaluate index expression, which must be some integral type
                let index = self.eval(h, ctx, indexing.index)?;
                // Mutable reference to the subject
                let subject;
                match &h[indexing.subject] {
                    Expression::Variable(var) => {
                        let var = var.declaration.unwrap();
                        subject = self.map.get_mut(&var).unwrap();
                    }
                    _ => unreachable!(),
                }
                match subject.set(&index, &value) {
                    Some(value) => Ok(value),
                    None => Err(EvalContinuation::Inconsistent),
                }
            }
            _ => unimplemented!("{:?}", h[lexpr]),
        }
    }
    fn get(&mut self, h: &Heap, ctx: &mut EvalContext, rexpr: ExpressionId) -> EvalResult {
        match &h[rexpr] {
            Expression::Variable(var) => {
                let var = var.declaration.unwrap();
                let value = self
                    .map
                    .get(&var)
                    .expect(&format!("Uninitialized variable {:?}", h[h[var].identifier()]));
                Ok(value.clone())
            }
            Expression::Indexing(indexing) => {
                // Evaluate index expression, which must be some integral type
                let index = self.eval(h, ctx, indexing.index)?;
                // Reference to subject
                let subject;
                match &h[indexing.subject] {
                    Expression::Variable(var) => {
                        let var = var.declaration.unwrap();
                        subject = self.map.get(&var).unwrap();
                    }
                    _ => unreachable!(),
                }
                match subject.get(&index) {
                    Some(value) => Ok(value),
                    None => Err(EvalContinuation::Inconsistent),
                }
            }
            Expression::Select(selecting) => {
                // Reference to subject
                let subject;
                match &h[selecting.subject] {
                    Expression::Variable(var) => {
                        let var = var.declaration.unwrap();
                        subject = self.map.get(&var).unwrap();
                    }
                    _ => unreachable!(),
                }
                match subject.length() {
                    Some(value) => Ok(value),
                    None => Err(EvalContinuation::Inconsistent),
                }
            }
            _ => unimplemented!("{:?}", h[rexpr]),
        }
    }
    fn eval(&mut self, h: &Heap, ctx: &mut EvalContext, expr: ExpressionId) -> EvalResult {
        match &h[expr] {
            Expression::Assignment(expr) => {
                let value = self.eval(h, ctx, expr.right)?;
                match expr.operation {
                    AssignmentOperator::Set => {
                        self.update(h, ctx, expr.left, value.clone())?;
                    }
                    AssignmentOperator::Added => {
                        let old = self.get(h, ctx, expr.left)?;
                        self.update(h, ctx, expr.left, old.plus(&value))?;
                    }
                    AssignmentOperator::Subtracted => {
                        let old = self.get(h, ctx, expr.left)?;
                        self.update(h, ctx, expr.left, old.minus(&value))?;
                    }
                    _ => unimplemented!("{:?}", expr),
                }
                Ok(value)
            }
            Expression::Conditional(expr) => {
                let test = self.eval(h, ctx, expr.test)?;
                if test.as_boolean().0 {
                    self.eval(h, ctx, expr.true_expression)
                } else {
                    self.eval(h, ctx, expr.false_expression)
                }
            }
            Expression::Binary(expr) => {
                let left = self.eval(h, ctx, expr.left)?;
                let right;
                match expr.operation {
                    BinaryOperator::LogicalAnd => {
                        if left.as_boolean().0 == false {
                            return Ok(left);
                        }
                        right = self.eval(h, ctx, expr.right)?;
                        right.as_boolean(); // panics if not a boolean
                        return Ok(right);
                    }
                    BinaryOperator::LogicalOr => {
                        if left.as_boolean().0 == true {
                            return Ok(left);
                        }
                        right = self.eval(h, ctx, expr.right)?;
                        right.as_boolean(); // panics if not a boolean
                        return Ok(right);
                    }
                    _ => {}
                }
                right = self.eval(h, ctx, expr.right)?;
                match expr.operation {
                    BinaryOperator::Equality => Ok(left.eq(&right)),
                    BinaryOperator::Inequality => Ok(left.neq(&right)),
                    BinaryOperator::LessThan => Ok(left.lt(&right)),
                    BinaryOperator::LessThanEqual => Ok(left.lte(&right)),
                    BinaryOperator::GreaterThan => Ok(left.gt(&right)),
                    BinaryOperator::GreaterThanEqual => Ok(left.gte(&right)),
                    BinaryOperator::Remainder => Ok(left.modulus(&right)),
                    _ => unimplemented!("{:?}", expr.operation),
                }
            }
            Expression::Unary(expr) => {
                let mut value = self.eval(h, ctx, expr.expression)?;
                match expr.operation {
                    UnaryOperation::PostIncrement => {
                        self.update(h, ctx, expr.expression, value.plus(&ONE))?;
                    }
                    UnaryOperation::PreIncrement => {
                        value = value.plus(&ONE);
                        self.update(h, ctx, expr.expression, value.clone())?;
                    }
                    UnaryOperation::PostDecrement => {
                        self.update(h, ctx, expr.expression, value.minus(&ONE))?;
                    }
                    UnaryOperation::PreDecrement => {
                        value = value.minus(&ONE);
                        self.update(h, ctx, expr.expression, value.clone())?;
                    }
                    _ => unimplemented!(),
                }
                Ok(value)
            }
            Expression::Indexing(expr) => self.get(h, ctx, expr.this.upcast()),
            Expression::Slicing(expr) => unimplemented!(),
            Expression::Select(expr) => self.get(h, ctx, expr.this.upcast()),
            Expression::Array(expr) => {
                let mut elements = Vec::new();
                for &elem in expr.elements.iter() {
                    elements.push(self.eval(h, ctx, elem)?);
                }
                todo!()
            }
            Expression::Constant(expr) => Ok(Value::from_constant(&expr.value)),
            Expression::Call(expr) => match expr.method {
                Method::Create => {
                    assert_eq!(1, expr.arguments.len());
                    let length = self.eval(h, ctx, expr.arguments[0])?;
                    Ok(Value::create_message(length))
                }
                Method::Fires => {
                    assert_eq!(1, expr.arguments.len());
                    let value = self.eval(h, ctx, expr.arguments[0])?;
                    match ctx.fires(value.clone()) {
                        None => Err(EvalContinuation::BlockFires(value)),
                        Some(result) => Ok(result),
                    }
                }
                Method::Get => {
                    assert_eq!(1, expr.arguments.len());
                    let value = self.eval(h, ctx, expr.arguments[0])?;
                    match ctx.get(value.clone()) {
                        None => Err(EvalContinuation::BlockGet(value)),
                        Some(result) => Ok(result),
                    }
                }
                Method::Symbolic(symbol) => unimplemented!(),
            },
            Expression::Variable(expr) => self.get(h, ctx, expr.this.upcast()),
        }
    }
}

type EvalResult = Result<Value, EvalContinuation>;
pub enum EvalContinuation {
    Stepping,
    Inconsistent,
    Terminal,
    SyncBlockStart,
    SyncBlockEnd,
    NewComponent(DeclarationId, Vec<Value>),
    BlockFires(Value),
    BlockGet(Value),
    Put(Value, Value),
}

#[derive(Debug, Clone)]
pub struct Prompt {
    definition: DefinitionId,
    store: Store,
    position: Option<StatementId>,
}

impl Prompt {
    pub fn new(h: &Heap, def: DefinitionId, args: &Vec<Value>) -> Self {
        let mut prompt =
            Prompt { definition: def, store: Store::new(), position: Some((&h[def]).body()) };
        prompt.set_arguments(h, args);
        prompt
    }
    fn set_arguments(&mut self, h: &Heap, args: &Vec<Value>) {
        let def = &h[self.definition];
        let params = def.parameters();
        assert_eq!(params.len(), args.len());
        for (param, value) in params.iter().zip(args.iter()) {
            let hparam = &h[*param];
            let type_annot = &h[hparam.type_annotation];
            assert!(value.is_type_compatible(&type_annot.the_type));
            self.store.initialize(h, param.upcast(), value.clone());
        }
    }
    pub fn step(&mut self, h: &Heap, ctx: &mut EvalContext) -> EvalResult {
        if self.position.is_none() {
            return Err(EvalContinuation::Terminal);
        }
        let stmt = &h[self.position.unwrap()];
        match stmt {
            Statement::Block(stmt) => {
                // Continue to first statement
                self.position = Some(stmt.first());
                Err(EvalContinuation::Stepping)
            }
            Statement::Local(stmt) => {
                match stmt {
                    LocalStatement::Memory(stmt) => {
                        // Evaluate initial expression
                        let value = self.store.eval(h, ctx, stmt.initial)?;
                        // Update store
                        self.store.initialize(h, stmt.variable.upcast(), value);
                    }
                    LocalStatement::Channel(stmt) => {
                        let [from, to] = ctx.new_channel();
                        // Store the values in the declared variables
                        self.store.initialize(h, stmt.from.upcast(), from);
                        self.store.initialize(h, stmt.to.upcast(), to);
                    }
                }
                // Continue to next statement
                self.position = stmt.next();
                Err(EvalContinuation::Stepping)
            }
            Statement::Skip(stmt) => {
                // Continue to next statement
                self.position = stmt.next;
                Err(EvalContinuation::Stepping)
            }
            Statement::Labeled(stmt) => {
                // Continue to next statement
                self.position = Some(stmt.body);
                Err(EvalContinuation::Stepping)
            }
            Statement::If(stmt) => {
                // Evaluate test
                let value = self.store.eval(h, ctx, stmt.test)?;
                // Continue with either branch
                if value.as_boolean().0 {
                    self.position = Some(stmt.true_body);
                } else {
                    self.position = Some(stmt.false_body);
                }
                Err(EvalContinuation::Stepping)
            }
            Statement::EndIf(stmt) => {
                // Continue to next statement
                self.position = stmt.next;
                Err(EvalContinuation::Stepping)
            }
            Statement::While(stmt) => {
                // Evaluate test
                let value = self.store.eval(h, ctx, stmt.test)?;
                // Either continue with body, or go to next
                if value.as_boolean().0 {
                    self.position = Some(stmt.body);
                } else {
                    self.position = stmt.next.map(|x| x.upcast());
                }
                Err(EvalContinuation::Stepping)
            }
            Statement::EndWhile(stmt) => {
                // Continue to next statement
                self.position = stmt.next;
                Err(EvalContinuation::Stepping)
            }
            Statement::Synchronous(stmt) => {
                // Continue to next statement, and signal upward
                self.position = Some(stmt.body);
                Err(EvalContinuation::SyncBlockStart)
            }
            Statement::EndSynchronous(stmt) => {
                // Continue to next statement, and signal upward
                self.position = stmt.next;
                Err(EvalContinuation::SyncBlockEnd)
            }
            Statement::Break(stmt) => {
                // Continue to end of while
                self.position = stmt.target.map(EndWhileStatementId::upcast);
                Err(EvalContinuation::Stepping)
            }
            Statement::Continue(stmt) => {
                // Continue to beginning of while
                self.position = stmt.target.map(WhileStatementId::upcast);
                Err(EvalContinuation::Stepping)
            }
            Statement::Assert(stmt) => {
                // Evaluate expression
                let value = self.store.eval(h, ctx, stmt.expression)?;
                if value.as_boolean().0 {
                    // Continue to next statement
                    self.position = stmt.next;
                    Err(EvalContinuation::Stepping)
                } else {
                    // Assertion failed: inconsistent
                    Err(EvalContinuation::Inconsistent)
                }
            }
            Statement::Return(stmt) => {
                // Evaluate expression
                let value = self.store.eval(h, ctx, stmt.expression)?;
                // Done with evaluation
                Ok(value)
            }
            Statement::Goto(stmt) => {
                // Continue to target
                self.position = stmt.target.map(|x| x.upcast());
                Err(EvalContinuation::Stepping)
            }
            Statement::New(stmt) => {
                let expr = &h[stmt.expression];
                let mut args = Vec::new();
                for &arg in expr.arguments.iter() {
                    let value = self.store.eval(h, ctx, arg)?;
                    args.push(value);
                }
                self.position = stmt.next;
                Err(EvalContinuation::NewComponent(expr.declaration.unwrap(), args))
            }
            Statement::Put(stmt) => {
                // Evaluate port and message
                let port = self.store.eval(h, ctx, stmt.port)?;
                let message = self.store.eval(h, ctx, stmt.message)?;
                // Continue to next statement
                self.position = stmt.next;
                // Signal the put upwards
                Err(EvalContinuation::Put(port, message))
            }
            Statement::Expression(stmt) => {
                // Evaluate expression
                let value = self.store.eval(h, ctx, stmt.expression)?;
                // Continue to next statement
                self.position = stmt.next;
                Err(EvalContinuation::Stepping)
            }
        }
    }
    fn compute_function(h: &Heap, fun: FunctionId, args: &Vec<Value>) -> Option<Value> {
        let mut prompt = Self::new(h, fun.upcast(), args);
        let mut context = EvalContext::None;
        loop {
            let result = prompt.step(h, &mut context);
            match result {
                Ok(val) => return Some(val),
                Err(cont) => match cont {
                    EvalContinuation::Stepping => continue,
                    EvalContinuation::Inconsistent => return None,
                    // Functions never terminate without returning
                    EvalContinuation::Terminal => unreachable!(),
                    // Functions never encounter any blocking behavior
                    EvalContinuation::SyncBlockStart => unreachable!(),
                    EvalContinuation::SyncBlockEnd => unreachable!(),
                    EvalContinuation::NewComponent(_, _) => unreachable!(),
                    EvalContinuation::BlockFires(val) => unreachable!(),
                    EvalContinuation::BlockGet(val) => unreachable!(),
                    EvalContinuation::Put(port, msg) => unreachable!(),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test_generator;

    use std::fs::File;
    use std::io::Read;
    use std::path::Path;
    use test_generator::test_resources;

    use super::*;

    #[test_resources("testdata/eval/positive/*.pdl")]
    fn batch1(resource: &str) {
        let path = Path::new(resource);
        let expect = path.with_extension("txt");
        let mut heap = Heap::new();
        let mut source = InputSource::from_file(&path).unwrap();
        let mut parser = Parser::new(&mut source);
        let pd = parser.parse(&mut heap).unwrap();
        let def = heap[pd].get_definition_ident(&heap, b"test").unwrap();
        let fun = heap[def].as_function().this;
        let args = Vec::new();
        let result = Prompt::compute_function(&heap, fun, &args).unwrap();
        let valstr: String = format!("{}", result);
        println!("{}", valstr);

        let mut cev: Vec<u8> = Vec::new();
        let mut f = File::open(expect).unwrap();
        f.read_to_end(&mut cev).unwrap();
        let lavstr = String::from_utf8_lossy(&cev);
        println!("{}", lavstr);

        assert_eq!(valstr, lavstr);
    }
}
