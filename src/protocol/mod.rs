mod ast;
mod eval;
pub mod inputsource;
mod lexer;
mod library;
mod parser;

use crate::common::*;
use crate::protocol::ast::*;
use crate::protocol::eval::*;
use crate::protocol::inputsource::*;
use crate::protocol::parser::*;
use std::hint::unreachable_unchecked;

pub struct ProtocolDescriptionImpl {
    heap: Heap,
    source: InputSource,
    root: RootId,
    main: ComponentId,
}

impl std::fmt::Debug for ProtocolDescriptionImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Protocol")
    }
}

impl ProtocolDescription for ProtocolDescriptionImpl {
    type S = ComponentStateImpl;

    fn parse(buffer: &[u8]) -> Result<Self, String> {
        let mut heap = Heap::new();
        let mut source = InputSource::from_buffer(buffer).unwrap();
        let mut parser = Parser::new(&mut source);
        match parser.parse(&mut heap) {
            Ok(root) => {
                // Find main definition (grammar rule ensures this exists)
                let sym = heap.get_external_identifier(b"main");
                let def = heap[root].get_definition(&heap, sym.upcast()).unwrap();
                let main = heap[def].as_component().this();
                return Ok(ProtocolDescriptionImpl { heap, source, root, main });
            }
            Err(err) => {
                let mut vec: Vec<u8> = Vec::new();
                err.write(&source, &mut vec).unwrap();
                Err(String::from_utf8_lossy(&vec).to_string())
            }
        }
    }
    fn main_interface_polarities(&self) -> Vec<Polarity> {
        let def = &self.heap[self.main];
        let mut result = Vec::new();
        for &param in def.parameters().iter() {
            let param = &self.heap[param];
            let type_annot = &self.heap[param.type_annotation];
            let ptype = &type_annot.the_type.primitive;
            if ptype == &PrimitiveType::Input {
                result.push(Polarity::Getter)
            } else if ptype == &PrimitiveType::Output {
                result.push(Polarity::Putter)
            } else {
                unreachable!()
            }
        }
        result
    }
    fn new_main_component(&self, interface: &[Key]) -> ComponentStateImpl {
        let mut args = Vec::new();
        for (&x, y) in interface.iter().zip(self.main_interface_polarities()) {
            match y {
                Polarity::Getter => args.push(Value::Input(InputValue(x))),
                Polarity::Putter => args.push(Value::Output(OutputValue(x))),
            }
        }
        ComponentStateImpl { prompt: Prompt::new(&self.heap, self.main.upcast(), &args) }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStateImpl {
    prompt: Prompt,
}
impl ComponentState for ComponentStateImpl {
    type D = ProtocolDescriptionImpl;

    fn pre_sync_run<C: MonoContext<D = ProtocolDescriptionImpl, S = Self>>(
        &mut self,
        context: &mut C,
        pd: &ProtocolDescriptionImpl,
    ) -> MonoBlocker {
        let mut context = EvalContext::Mono(context);
        loop {
            let result = self.prompt.step(&pd.heap, &mut context);
            match result {
                // In component definitions, there are no return statements
                Ok(_) => unreachable!(),
                Err(cont) => match cont {
                    EvalContinuation::Stepping => continue,
                    EvalContinuation::Inconsistent => return MonoBlocker::Inconsistent,
                    EvalContinuation::Terminal => return MonoBlocker::ComponentExit,
                    EvalContinuation::SyncBlockStart => return MonoBlocker::SyncBlockStart,
                    // Not possible to end sync block if never entered one
                    EvalContinuation::SyncBlockEnd => unreachable!(),
                    EvalContinuation::NewComponent(args) => {
                        todo!();
                        continue;
                    }
                    // Outside synchronous blocks, no fires/get/put happens
                    EvalContinuation::BlockFires(val) => unreachable!(),
                    EvalContinuation::BlockGet(val) => unreachable!(),
                    EvalContinuation::Put(port, msg) => unreachable!(),
                },
            }
        }
    }

    fn sync_run<C: PolyContext<D = ProtocolDescriptionImpl>>(
        &mut self,
        context: &mut C,
        pd: &ProtocolDescriptionImpl,
    ) -> PolyBlocker {
        let mut context = EvalContext::Poly(context);
        loop {
            let result = self.prompt.step(&pd.heap, &mut context);
            match result {
                // Inside synchronous blocks, there are no return statements
                Ok(_) => unreachable!(),
                Err(cont) => match cont {
                    EvalContinuation::Stepping => continue,
                    EvalContinuation::Inconsistent => return PolyBlocker::Inconsistent,
                    // First need to exit synchronous block before definition may end
                    EvalContinuation::Terminal => unreachable!(),
                    // No nested synchronous blocks
                    EvalContinuation::SyncBlockStart => unreachable!(),
                    EvalContinuation::SyncBlockEnd => return PolyBlocker::SyncBlockEnd,
                    // Not possible to create component in sync block
                    EvalContinuation::NewComponent(args) => unreachable!(),
                    EvalContinuation::BlockFires(port) => match port {
                        Value::Output(OutputValue(key)) => {
                            return PolyBlocker::CouldntCheckFiring(key);
                        }
                        Value::Input(InputValue(key)) => {
                            return PolyBlocker::CouldntCheckFiring(key);
                        }
                        _ => unreachable!(),
                    },
                    EvalContinuation::BlockGet(port) => match port {
                        Value::Output(OutputValue(key)) => {
                            return PolyBlocker::CouldntReadMsg(key);
                        }
                        Value::Input(InputValue(key)) => {
                            return PolyBlocker::CouldntReadMsg(key);
                        }
                        _ => unreachable!(),
                    },
                    EvalContinuation::Put(port, message) => {
                        let key;
                        match port {
                            Value::Output(OutputValue(the_key)) => {
                                key = the_key;
                            }
                            Value::Input(InputValue(the_key)) => {
                                key = the_key;
                            }
                            _ => unreachable!(),
                        }
                        let payload;
                        match message {
                            Value::Message(MessageValue(None)) => {
                                // Putting a null message is inconsistent
                                return PolyBlocker::Inconsistent;
                            }
                            Value::Message(MessageValue(Some(buffer))) => {
                                // Create a copy of the payload
                                payload = buffer.clone();
                            }
                            _ => unreachable!(),
                        }
                        return PolyBlocker::PutMsg(key, payload);
                    }
                },
            }
        }
    }
}

pub enum EvalContext<'a> {
    Mono(&'a mut dyn MonoContext<D = ProtocolDescriptionImpl, S = ComponentStateImpl>),
    Poly(&'a mut dyn PolyContext<D = ProtocolDescriptionImpl>),
    None,
}
impl EvalContext<'_> {
    fn random(&mut self) -> LongValue {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => todo!(),
            EvalContext::Poly(context) => unreachable!(),
        }
    }
    fn channel(&mut self) -> (Value, Value) {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => unreachable!(),
            EvalContext::Poly(context) => todo!(),
        }
    }
    fn fires(&mut self, port: Value) -> Option<Value> {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => unreachable!(),
            EvalContext::Poly(context) => match port {
                Value::Output(OutputValue(key)) => context.is_firing(key).map(Value::from),
                Value::Input(InputValue(key)) => context.is_firing(key).map(Value::from),
                _ => unreachable!(),
            },
        }
    }
    fn get(&mut self, port: Value) -> Option<Value> {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => unreachable!(),
            EvalContext::Poly(context) => match port {
                Value::Output(OutputValue(key)) => {
                    context.read_msg(key).map(Value::receive_message)
                }
                Value::Input(InputValue(key)) => context.read_msg(key).map(Value::receive_message),
                _ => unreachable!(),
            },
        }
    }
}
