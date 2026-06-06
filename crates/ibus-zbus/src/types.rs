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
use zvariant::{OwnedValue, Type, Value};

use crate::consts::{
    ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, ORIENTATION_SYSTEM, PROP_STATE_CHECKED,
    PROP_STATE_UNCHECKED, PROP_TYPE_MENU, PROP_TYPE_RADIO,
};

/// `a{sv}` — the attachments map every IBus object carries.
type Attachments = HashMap<String, OwnedValue>;

/// Wrap an IBus object as the variant (`v`) IBus expects on the wire.
fn variant<T: Into<Value<'static>>>(value: T) -> OwnedValue {
    let value: Value<'static> = value.into();
    OwnedValue::try_from(value).expect("IBus object converts to an owned variant")
}

fn empty_attachments() -> Attachments {
    HashMap::new()
}

/// `IBusAttribute` — a styling run over preedit text. Signature `(sa{sv}uuuu)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
pub struct IBusAttribute {
    pub name: String,
    pub attachments: Attachments,
    pub kind: u32,
    pub value: u32,
    pub start_index: u32,
    pub end_index: u32,
}

/// `IBusAttrList` — a list of attributes. Signature `(sa{sv}av)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
pub struct IBusAttrList {
    pub name: String,
    pub attachments: Attachments,
    pub attributes: Vec<OwnedValue>,
}

/// `IBusText` — displayable text plus its attributes. Signature `(sa{sv}sv)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
pub struct IBusText {
    pub name: String,
    pub attachments: Attachments,
    pub text: String,
    pub attr_list: OwnedValue,
}

/// `IBusProperty` — a property-panel entry. Signature `(sa{sv}suvsvbbuvv)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
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
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
pub struct IBusPropList {
    pub name: String,
    pub attachments: Attachments,
    pub property_list: Vec<OwnedValue>,
}

/// `IBusLookupTable` — candidate list. Signature `(sa{sv}uubbiavav)`.
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
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
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
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
#[derive(Debug, Clone, Type, Serialize, Deserialize, Value, OwnedValue)]
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

impl IBusAttribute {
    pub fn new(kind: u32, value: u32, start_index: u32, end_index: u32) -> Self {
        Self {
            name: "IBusAttribute".into(),
            attachments: empty_attachments(),
            kind,
            value,
            start_index,
            end_index,
        }
    }
}

impl IBusAttrList {
    pub fn new() -> Self {
        Self::with(Vec::new())
    }

    pub fn with(attributes: Vec<IBusAttribute>) -> Self {
        Self {
            name: "IBusAttrList".into(),
            attachments: empty_attachments(),
            attributes: attributes.into_iter().map(variant).collect(),
        }
    }
}

impl Default for IBusAttrList {
    fn default() -> Self {
        Self::new()
    }
}

impl IBusText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            name: "IBusText".into(),
            attachments: empty_attachments(),
            text: text.into(),
            attr_list: variant(IBusAttrList::new()),
        }
    }

    /// Text with a single-underline attribute spanning the whole string.
    pub fn with_underline(text: impl Into<String>) -> Self {
        let text = text.into();
        let end = text.chars().count() as u32;
        let attr = IBusAttribute::new(ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, 0, end);
        Self {
            name: "IBusText".into(),
            attachments: empty_attachments(),
            text,
            attr_list: variant(IBusAttrList::with(vec![attr])),
        }
    }
}

impl IBusProperty {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: impl Into<String>,
        kind: u32,
        label: &str,
        icon: impl Into<String>,
        tooltip: &str,
        sensitive: bool,
        visible: bool,
        state: u32,
    ) -> Self {
        Self {
            name: "IBusProperty".into(),
            attachments: empty_attachments(),
            key: key.into(),
            kind,
            label: variant(IBusText::new(label)),
            icon: icon.into(),
            tooltip: variant(IBusText::new(tooltip)),
            sensitive,
            visible,
            state,
            sub_props: variant(IBusPropList::new(Vec::new())),
            symbol: variant(IBusText::new("")),
        }
    }
}

impl IBusPropList {
    pub fn new(properties: Vec<IBusProperty>) -> Self {
        Self {
            name: "IBusPropList".into(),
            attachments: empty_attachments(),
            property_list: properties.into_iter().map(variant).collect(),
        }
    }
}

impl IBusLookupTable {
    pub fn new() -> Self {
        Self {
            name: "IBusLookupTable".into(),
            attachments: empty_attachments(),
            page_size: 5,
            cursor_pos: 0,
            cursor_visible: true,
            round: false,
            orientation: ORIENTATION_SYSTEM,
            candidates: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn append_candidate(&mut self, text: &str) {
        self.candidates.push(variant(IBusText::new(text)));
    }

    pub fn append_label(&mut self, label: &str) {
        self.labels.push(variant(IBusText::new(label)));
    }
}

impl Default for IBusLookupTable {
    fn default() -> Self {
        Self::new()
    }
}

impl IBusProperty {
    /// A radio-button property, checked or not.
    pub fn radio(key: impl Into<String>, label: &str, checked: bool) -> Self {
        let state = if checked {
            PROP_STATE_CHECKED
        } else {
            PROP_STATE_UNCHECKED
        };
        Self::new(key, PROP_TYPE_RADIO, label, "", "", true, true, state)
    }

    /// A submenu property whose children are `properties`.
    pub fn menu(key: impl Into<String>, label: &str, properties: Vec<IBusProperty>) -> Self {
        let mut prop = Self::new(
            key,
            PROP_TYPE_MENU,
            label,
            "",
            "",
            true,
            true,
            PROP_STATE_UNCHECKED,
        );
        prop.sub_props = variant(IBusPropList::new(properties));
        prop
    }
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

    #[test]
    fn text_constructors() {
        let t = IBusText::new("việt");
        assert_eq!(t.name, "IBusText");
        assert_eq!(t.text, "việt");

        let u = IBusText::with_underline("việt");
        // Round-trip the nested attr list back out of its variant.
        let list = IBusAttrList::try_from(u.attr_list).unwrap();
        assert_eq!(list.attributes.len(), 1);
        let attr = IBusAttribute::try_from(list.attributes[0].clone()).unwrap();
        assert_eq!(attr.kind, ATTR_TYPE_UNDERLINE);
        assert_eq!(attr.end_index, 4); // four chars in "việt"
    }

    #[test]
    fn property_menu_roundtrips() {
        let menu = IBusProperty::menu(
            "Keyboard",
            "Keyboard",
            vec![
                IBusProperty::radio("Keyboard.QWERTY", "QWERTY", true),
                IBusProperty::radio("Keyboard.AZERTY", "AZERTY", false),
            ],
        );
        let list = IBusPropList::new(vec![menu]);
        // This is exactly what register_properties sends (wrapped in a variant).
        let v = variant(list);
        let back = IBusPropList::try_from(v).unwrap();
        assert_eq!(back.property_list.len(), 1);
    }

    #[test]
    fn lookup_table_defaults_and_append() {
        let mut lt = IBusLookupTable::new();
        assert_eq!(lt.page_size, 5);
        assert!(lt.cursor_visible);
        assert_eq!(lt.orientation, ORIENTATION_SYSTEM);
        lt.append_candidate("a");
        lt.append_label("1");
        assert_eq!(lt.candidates.len(), 1);
        assert_eq!(lt.labels.len(), 1);
    }
}
