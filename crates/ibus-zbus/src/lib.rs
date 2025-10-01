//! Engine-side IBus binding for ibus-bambusa, built on zbus.
//!
//! Currently provides the IBus wire types; the bus connection and the
//! `Factory`/`Engine` interfaces are added on top of these.

mod types;

pub use types::{
    IBusAttrList, IBusAttribute, IBusComponent, IBusEngineDesc, IBusLookupTable, IBusPropList,
    IBusProperty, IBusText,
};
