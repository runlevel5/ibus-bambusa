//! The IBus engines we expose, one per typing method.
//!
//! Each entry is a separate IBus input source, so GNOME switches between
//! methods (and remembers them per window) with its own Super+Space mechanism —
//! we just map the engine name IBus hands `CreateEngine` back to a method.

/// VNI rules for a QWERTY keyboard (digit triggers).
pub const VNI: &str = "VNI";
/// VNI rules for an AZERTY keyboard (the French-keyboard variant).
pub const VNI_AZERTY: &str = "VNI Bàn phím tiếng Pháp";

/// `(ibus engine name, bambusa-core input method name)`. The method names must
/// match the built-in methods exactly.
pub const ENGINES: &[(&str, &str)] = &[
    ("Bambusa", "Telex"),
    ("BambusaTelexW", "Telex W"),
    ("BambusaTelex2", "Telex 2"),
    ("BambusaVNI", VNI),
    ("BambusaVIQR", "VIQR"),
    ("BambusaMicrosoft", "Microsoft layout"),
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
        assert_eq!(method_for_engine("Bambusa"), "Telex");
        assert_eq!(method_for_engine("BambusaVNI"), "VNI");
        assert_eq!(method_for_engine("BambusaMicrosoft"), "Microsoft layout");
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
