//! Engine-side IBus binding for ibus-bambusa, built on zbus.
//!
//! Currently provides the IBus wire types and constants; the bus connection
//! and the `Factory`/`Engine` interfaces are added on top of these.

pub mod address;
pub mod consts;
mod engine;
mod factory;
mod handler;
mod types;

pub use engine::EngineInterface;
pub use factory::Factory;
pub use handler::{Action, EchoHandler, EngineHandler};
pub use types::{
    IBusAttrList, IBusAttribute, IBusComponent, IBusEngineDesc, IBusLookupTable, IBusPropList,
    IBusProperty, IBusText,
};
