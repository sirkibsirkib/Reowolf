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
                return Ok(ProtocolDescriptionImpl { heap, source, root });
            }
            Err(err) => {
                let mut vec: Vec<u8> = Vec::new();
                err.write(&source, &mut vec).unwrap();
                Err(String::from_utf8_lossy(&vec).to_string())
            }
        }
    }
    fn component_polarities(&self, identifier: &[u8]) -> Result<Vec<Polarity>, MainComponentErr> {
        let h = &self.heap;
        let root = &h[self.root];
        let def = root.get_definition_ident(h, identifier);
        if def.is_none() {
            return Err(MainComponentErr::NoSuchComponent);
        }
        let def = &h[def.unwrap()];
        if !def.is_component() {
            return Err(MainComponentErr::NoSuchComponent);
        }
        for &param in def.parameters().iter() {
            let param = &h[param];
            let type_annot = &h[param.type_annotation];
            if type_annot.the_type.array {
                return Err(MainComponentErr::NonPortTypeParameters);
            }
            match type_annot.the_type.primitive {
                PrimitiveType::Input | PrimitiveType::Output => continue,
                _ => {
                    return Err(MainComponentErr::NonPortTypeParameters);
                }
            }
        }
        let mut result = Vec::new();
        for &param in def.parameters().iter() {
            let param = &h[param];
            let type_annot = &h[param.type_annotation];
            let ptype = &type_annot.the_type.primitive;
            if ptype == &PrimitiveType::Input {
                result.push(Polarity::Getter)
            } else if ptype == &PrimitiveType::Output {
                result.push(Polarity::Putter)
            } else {
                unreachable!()
            }
        }
        Ok(result)
    }
    fn new_main_component(&self, identifier: &[u8], ports: &[Key]) -> ComponentStateImpl {
        let mut args = Vec::new();
        for (&x, y) in ports.iter().zip(self.component_polarities(identifier).unwrap()) {
            match y {
                Polarity::Getter => args.push(Value::Input(InputValue(x))),
                Polarity::Putter => args.push(Value::Output(OutputValue(x))),
            }
        }
        let h = &self.heap;
        let root = &h[self.root];
        let def = root.get_definition_ident(h, identifier).unwrap();
        ComponentStateImpl { prompt: Prompt::new(h, def, &args) }
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
                    EvalContinuation::NewComponent(decl, args) => {
                        // Look up definition (TODO for now, assume it is a definition)
                        let h = &pd.heap;
                        let def = h[decl].as_defined().definition;
                        println!("Create component: {}",  String::from_utf8_lossy(h[h[def].identifier()].ident()));
                        let init_state = ComponentStateImpl { prompt: Prompt::new(h, def, &args) };
                        context.new_component(&args, init_state);
                        // Continue stepping
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
                    EvalContinuation::NewComponent(_, _) => unreachable!(),
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
            EvalContext::Poly(_) => unreachable!(),
        }
    }
    fn new_component(&mut self, args: &[Value], init_state: ComponentStateImpl) -> () {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => {
                let mut moved_keys = HashSet::new();
                for arg in args.iter() {
                    match arg {
                        Value::Output(OutputValue(key)) => { moved_keys.insert(*key); }
                        Value::Input(InputValue(key)) => { moved_keys.insert(*key); }
                        _ => {}
                    }
                }
                context.new_component(moved_keys, init_state)
            }
            EvalContext::Poly(_) => unreachable!(),
        }
    }
    fn new_channel(&mut self) -> [Value; 2] {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(context) => {
                let [from, to] = context.new_channel();
                let from = Value::Output(OutputValue(from));
                let to = Value::Input(InputValue(to));
                return [from, to];
            }
            EvalContext::Poly(_) => unreachable!()
        }
    }
    fn fires(&mut self, port: Value) -> Option<Value> {
        match self {
            EvalContext::None => unreachable!(),
            EvalContext::Mono(_) => unreachable!(),
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
            EvalContext::Mono(_) => unreachable!(),
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
