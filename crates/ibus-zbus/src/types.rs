//! The IBus wire types.
//!
//! Every IBus object is a D-Bus structure whose first two members are a type
//! name string and an `a{sv}` attachments dict, followed by the payload. On
//! the bus they travel wrapped in a variant. Field order here is what defines
//! the D-Bus signature, so it must match IBus exactly — the tests below assert
//! each signature so a reordering or wrong type fails at `cargo test` rather
//! than being silently dropped by IBus at runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvariant::{OwnedValue, Type};

/// `a{sv}` — the attachments map every IBus object carries.
type Attachments = HashMap<String, OwnedValue>;

/// `IBusAttribute` — a styling run over preedit text. Signature `(sa{sv}uuuu)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusAttribute {
    pub name: String,
    pub attachments: Attachments,
    pub kind: u32,
    pub value: u32,
    pub start_index: u32,
    pub end_index: u32,
}

/// `IBusAttrList` — a list of attributes. Signature `(sa{sv}av)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusAttrList {
    pub name: String,
    pub attachments: Attachments,
    pub attributes: Vec<OwnedValue>,
}

/// `IBusText` — displayable text plus its attributes. Signature `(sa{sv}sv)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusText {
    pub name: String,
    pub attachments: Attachments,
    pub text: String,
    pub attr_list: OwnedValue,
}

/// `IBusProperty` — a property-panel entry. Signature `(sa{sv}suvsvbbuvv)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusProperty {
    pub name: String,
    pub attachments: Attachments,
    pub key: String,
    pub kind: u32,
    pub label: OwnedValue,
    pub icon: String,
    pub tooltip: OwnedValue,
    pub sensitive: bool,
    pub visible: bool,
    pub state: u32,
    pub sub_props: OwnedValue,
    pub symbol: OwnedValue,
}

/// `IBusPropList` — a list of properties. Signature `(sa{sv}av)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusPropList {
    pub name: String,
    pub attachments: Attachments,
    pub property_list: Vec<OwnedValue>,
}

/// `IBusLookupTable` — candidate list. Signature `(sa{sv}uubbiavav)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusLookupTable {
    pub name: String,
    pub attachments: Attachments,
    pub page_size: u32,
    pub cursor_pos: u32,
    pub cursor_visible: bool,
    pub round: bool,
    pub orientation: i32,
    pub candidates: Vec<OwnedValue>,
    pub labels: Vec<OwnedValue>,
}

/// `IBusComponent` — component registration. Signature `(sa{sv}ssssssssavav)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusComponent {
    pub name: String,
    pub attachments: Attachments,
    pub component_name: String,
    pub description: String,
    pub version: String,
    pub license: String,
    pub author: String,
    pub homepage: String,
    pub exec: String,
    pub textdomain: String,
    pub observed_paths: Vec<OwnedValue>,
    pub engine_list: Vec<OwnedValue>,
}

/// `IBusEngineDesc` — engine descriptor. Signature `(sa{sv}ssssssssusssssss)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub struct IBusEngineDesc {
    pub name: String,
    pub attachments: Attachments,
    pub engine_name: String,
    pub long_name: String,
    pub description: String,
    pub language: String,
    pub license: String,
    pub author: String,
    pub icon: String,
    pub layout: String,
    pub rank: u32,
    pub hotkeys: String,
    pub symbol: String,
    pub setup: String,
    pub layout_variant: String,
    pub layout_option: String,
    pub version: String,
    pub textdomain: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_match_ibus() {
        assert_eq!(IBusAttribute::SIGNATURE.to_string(), "(sa{sv}uuuu)");
        assert_eq!(IBusAttrList::SIGNATURE.to_string(), "(sa{sv}av)");
        assert_eq!(IBusText::SIGNATURE.to_string(), "(sa{sv}sv)");
        assert_eq!(IBusProperty::SIGNATURE.to_string(), "(sa{sv}suvsvbbuvv)");
        assert_eq!(IBusPropList::SIGNATURE.to_string(), "(sa{sv}av)");
        assert_eq!(IBusLookupTable::SIGNATURE.to_string(), "(sa{sv}uubbiavav)");
        assert_eq!(IBusComponent::SIGNATURE.to_string(), "(sa{sv}ssssssssavav)");
        assert_eq!(
            IBusEngineDesc::SIGNATURE.to_string(),
            "(sa{sv}ssssssssusssssss)"
        );
    }
}
