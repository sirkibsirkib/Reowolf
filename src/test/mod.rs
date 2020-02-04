use core::fmt::Debug;

mod connector;
mod setup;

struct Panicked(Box<dyn std::any::Any>);
impl Debug for Panicked {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(str_slice) = self.0.downcast_ref::<&'static str>() {
            f.pad(str_slice)
        } else if let Some(string) = self.0.downcast_ref::<String>() {
            f.pad(string)
        } else {
            f.pad("Box<Any>")
        }
    }
}
fn handle(result: Result<(), std::boxed::Box<(dyn std::any::Any + std::marker::Send + 'static)>>) {
    match result {
        Ok(_) => {}
        Err(x) => {
            panic!("Worker panicked: {:?}", Panicked(x));
        }
    }
}
