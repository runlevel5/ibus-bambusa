//! Engine-side IBus binding for ibus-bambusa, built on zbus.
//!
//! Currently provides the IBus wire types and constants; the bus connection
//! and the `Factory`/`Engine` interfaces are added on top of these.

pub mod consts;
mod types;

pub use types::{
    IBusAttrList, IBusAttribute, IBusComponent, IBusEngineDesc, IBusLookupTable, IBusPropList,
    IBusProperty, IBusText,
};
