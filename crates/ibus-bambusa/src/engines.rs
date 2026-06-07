//! The IBus engines we expose, one per typing method.
//!
//! Each entry is a separate IBus input source, so GNOME switches between
//! methods (and remembers them per window) with its own Super+Space mechanism —
//! we just map the engine name IBus hands `CreateEngine` back to a method.
//!
//! Engine names share the `bambusa:` family prefix so GNOME resolves a single
//! `ibus-setup-bambusa.desktop` for the Settings "Preferences" button (it uses
//! the part of the name before the colon).

/// `(ibus engine name, bambusa-core input method name)`. The method names must
/// match the built-in methods exactly.
pub const ENGINES: &[(&str, &str)] = &[
    ("bambusa:telex", "Telex"),
    ("bambusa:telexw", "Telex W"),
    ("bambusa:telex2", "Telex 2"),
    ("bambusa:vni", "VNI"),
    ("bambusa:viqr", "VIQR"),
    ("bambusa:microsoft", "Microsoft layout"),
    ("bambusa:vni-azerty", "VNI (AZERTY)"),
    ("bambusa:vni-afnor", "VNI (AZERTY, AFNOR)"),
];

/// The input method for an IBus engine name, defaulting to Telex.
pub fn method_for_engine(name: &str) -> &'static str {
    ENGINES
        .iter()
        .find(|(engine, _)| *engine == name)
        .map_or("Telex", |(_, method)| *method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_engine_names_to_methods() {
        assert_eq!(method_for_engine("bambusa:telex"), "Telex");
        assert_eq!(method_for_engine("bambusa:vni"), "VNI");
        assert_eq!(method_for_engine("bambusa:vni-azerty"), "VNI (AZERTY)");
        assert_eq!(method_for_engine("anything-else"), "Telex");
    }

    #[test]
    fn every_exposed_method_is_built_in() {
        for (engine, method) in ENGINES {
            assert!(
                bambusa_core::parse_input_method(method).is_some(),
                "engine {engine} maps to unknown method {method}"
            );
        }
    }
}
