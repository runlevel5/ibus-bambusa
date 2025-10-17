//! IBus D-Bus interface names, object paths, and enum constants.

// Bus / object identifiers.
pub const IBUS_SERVICE: &str = "org.freedesktop.IBus";
pub const IBUS_PATH: &str = "/org/freedesktop/IBus";
pub const IFACE_IBUS: &str = "org.freedesktop.IBus";
pub const IFACE_SERVICE: &str = "org.freedesktop.IBus.Service";
pub const IFACE_ENGINE: &str = "org.freedesktop.IBus.Engine";
pub const IFACE_FACTORY: &str = "org.freedesktop.IBus.Factory";
pub const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
pub const DBUS_PROPERTIES: &str = "org.freedesktop.DBus.Properties";

// Property types (`IBusPropType`).
pub const PROP_TYPE_NORMAL: u32 = 0;
pub const PROP_TYPE_TOGGLE: u32 = 1;
pub const PROP_TYPE_RADIO: u32 = 2;
pub const PROP_TYPE_MENU: u32 = 3;
pub const PROP_TYPE_SEPARATOR: u32 = 4;

// Property states (`IBusPropState`).
pub const PROP_STATE_UNCHECKED: u32 = 0;
pub const PROP_STATE_CHECKED: u32 = 1;
pub const PROP_STATE_INCONSISTENT: u32 = 2;

// Text attribute types (`IBusAttrType`).
pub const ATTR_TYPE_NONE: u32 = 0;
pub const ATTR_TYPE_UNDERLINE: u32 = 1;
pub const ATTR_TYPE_FOREGROUND: u32 = 2;
pub const ATTR_TYPE_BACKGROUND: u32 = 3;

// Underline styles (`IBusAttrUnderline`).
pub const ATTR_UNDERLINE_NONE: u32 = 0;
pub const ATTR_UNDERLINE_SINGLE: u32 = 1;
pub const ATTR_UNDERLINE_DOUBLE: u32 = 2;
pub const ATTR_UNDERLINE_LOW: u32 = 3;
pub const ATTR_UNDERLINE_ERROR: u32 = 4;

// Lookup-table orientation (`IBusOrientation`).
pub const ORIENTATION_HORIZONTAL: i32 = 0;
pub const ORIENTATION_VERTICAL: i32 = 1;
pub const ORIENTATION_SYSTEM: i32 = 2;

// Preedit-text update mode.
pub const PREEDIT_CLEAR: u32 = 0;
pub const PREEDIT_COMMIT: u32 = 1;
