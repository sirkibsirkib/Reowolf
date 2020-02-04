#[macro_use]
mod macros;

mod common; // common to both
mod protocol; // hans' stuff
mod runtime; // chris' stuff

#[cfg(test)]
mod test;

pub use runtime::{errors, Connector, PortBinding};

#[cfg(feature = "ffi")]
pub use runtime::ffi;
