//! Behavioural corpus: every case the reference engine guarantees, driven
//! through the public API.

use bambusa_core::{BambusaEngine, EngineFlags, Mode, parse_input_method};

const VIE: Mode = Mode::VIETNAMESE;
const ENG: Mode = Mode::ENGLISH;

fn std_engine() -> BambusaEngine {
    // Telex 2 with the standard flag set is the reference's default engine.
    BambusaEngine::new(parse_input_method("Telex 2").unwrap(), EngineFlags::STD)
}

fn engine(im: &str, flags: EngineFlags) -> BambusaEngine {
    BambusaEngine::new(parse_input_method(im).unwrap(), flags)
}

#[test]
fn process_string_basics() {
    let mut e = std_engine();
    e.process_string("aw", VIE);
    assert_eq!(e.processed_string(VIE), "ă");

    e.reset();
    e.process_string("uw", VIE);
    e.process_string("o", VIE);
    e.process_string("w", VIE);
    assert_eq!(e.processed_string(VIE), "ươ");

    e.reset();
    e.process_string("chuaarn", VIE);
    assert_eq!(e.processed_string(VIE), "chuẩn");

    e.reset();
    e.process_string("giamaf", VIE);
    assert_eq!(e.processed_string(VIE), "giầm");
}

#[test]
fn process_dd() {
    let mut e = std_engine();
    e.process_string("dd", VIE);
    assert!(e.is_valid(false));

    e.reset();
    e.process_string("ddafi", VIE);
    assert_eq!(e.processed_string(VIE), "đài");

    e.reset();
    e.process_string("dd", VIE);
    assert!(e.is_valid(false));
    assert_eq!(e.processed_string(VIE), "đ");

    e.reset();
    e.process_string("SD", VIE);
    e.process_string("D", VIE);
    assert_eq!(e.processed_string(VIE), "SĐ");
}

#[test]
fn muoiwq_and_mootj() {
    let mut e = std_engine();
    e.process_string("Muoiwq", VIE);
    assert_eq!(e.processed_string(ENG), "Muoiwq");

    e.reset();
    e.process_string("mootj", VIE);
    assert_eq!(e.processed_string(VIE), "một");
}

#[test]
fn thuow_then_remove() {
    let mut e = std_engine();
    e.process_string("Thuow", VIE);
    assert_eq!(e.processed_string(VIE), "Thuơ");
    e.remove_last_char(true);
    assert_eq!(e.processed_string(VIE), "Thu");
}

#[test]
fn remove_last_char_sequences() {
    let mut e = std_engine();
    e.remove_last_char(true);
    e.process_string(" ", ENG);
    e.remove_last_char(true);
    e.process_string("loanj", VIE);
    assert_eq!(e.processed_string(VIE), "loạn");
    e.remove_last_char(true);
    assert_eq!(e.processed_string(VIE), "lọa");
    e.process_string(":", ENG);
    e.remove_last_char(true);
    assert_eq!(e.processed_string(VIE), "lọa");
}

#[test]
fn upper_string() {
    let mut e = std_engine();
    e.process_string("VIEETJ", VIE);
    assert_eq!(e.processed_string(VIE), "VIỆT");
    e.remove_last_char(false);
    assert_eq!(e.processed_string(VIE), "VIỆ");
    e.process_key('Q', VIE);
    assert_eq!(e.processed_string(ENG), "VIEEJQ");

    e.reset();
    e.process_string("IB", ENG);
    assert_eq!(e.processed_string(ENG), "IB");
}

#[test]
fn spelling_check_fallback() {
    let mut e = std_engine();
    e.process_string("noww", VIE);
    assert_eq!(e.processed_string(ENG), "noww");
    assert_eq!(e.processed_string(VIE), "now");

    e.reset();
    e.process_string("sawss", VIE);
    assert_eq!(e.processed_string(ENG), "sawss");

    e.reset();
    e.process_string("sawss", VIE);
    assert_eq!(e.processed_string(VIE), "săs");
}

#[test]
fn telex2_brackets() {
    let mut e = std_engine();
    e.process_string("t ]", ENG);
    e.process_string("a", VIE);
    assert_eq!(e.processed_string(VIE), "]a");

    e.reset();
    e.process_string("]]a", VIE);
    assert_eq!(e.processed_string(VIE), "]a");

    let mut e = std_engine();
    e.process_string("[", VIE);
    assert_eq!(e.processed_string(VIE), "ơ");
    e.reset();
    e.process_string("{", VIE);
    assert_eq!(e.processed_string(VIE), "Ơ");
}

#[test]
fn assorted_words() {
    let mut e = std_engine();
    e.process_string("wowfi", VIE);
    assert_eq!(e.processed_string(VIE), "ười");

    e.reset();
    e.process_string("hanhj", VIE);
    e.remove_last_char(true);
    assert_eq!(e.processed_string(VIE), "hạn");

    e.reset();
    e.process_string("catr", VIE);
    assert_eq!(e.processed_string(VIE), "catr");

    e.reset();
    e.process_string("toowi", VIE);
    assert_eq!(e.processed_string(VIE), "tơi");

    e.reset();
    e.process_string("aloo", VIE);
    assert_eq!(e.processed_string(VIE), "alô");

    e.reset();
    e.process_string("giw", VIE);
    assert!(e.is_valid(false));
}

#[test]
fn double_brackets_and_w() {
    let mut e = std_engine();
    e.process_string("[[", VIE);
    assert_eq!(e.processed_string(ENG), "[");

    e.reset();
    e.process_string("tooss", VIE);
    assert_eq!(e.processed_string(VIE), "tôs");
    e.reset();
    e.process_string("tosos", VIE);
    assert_eq!(e.processed_string(VIE), "tôs");

    e.reset();
    e.process_string("ww", VIE);
    assert_eq!(e.processed_string(ENG), "w");
    assert_eq!(e.processed_string(VIE), "w");

    e.reset();
    e.process_string("wiw", VIE);
    assert_eq!(e.processed_string(VIE), "uiw");
    assert_eq!(e.processed_string(ENG), "wiw");
}

#[test]
fn duwoi_and_refresh() {
    let mut e = std_engine();
    e.process_string("duwoi", VIE);
    assert_eq!(e.processed_string(VIE), "dươi");

    e.reset();
    e.process_string("reff", VIE);
    e.process_string("resh", ENG);
    assert_eq!(e.processed_string(ENG), "reffresh");
    assert_eq!(e.processed_string(VIE), "refresh");

    e.reset();
    e.process_string("reff", VIE);
    e.remove_last_char(true);
    e.process_key('f', VIE);
    assert_eq!(e.processed_string(VIE), "rè");
}

#[test]
fn dd_sequences_and_gi() {
    let mut e = std_engine();
    e.process_string("oddp", VIE);
    assert_eq!(e.processed_string(VIE), "ođp");

    e.reset();
    e.process_string("gis", VIE);
    e.process_string("a", VIE);
    assert_eq!(e.processed_string(VIE), "giá");

    e.reset();
    e.process_string("kimso", VIE);
    assert_eq!(e.processed_string(VIE), "kímo");

    e.reset();
    e.process_string("to", VIE);
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("toorr", VIE);
    assert_eq!(e.processed_string(VIE), "tôr");

    e.reset();
    e.process_string("tnoss", VIE);
    assert_eq!(e.processed_string(VIE), "tnos");
}

#[test]
fn ddawks_and_hieeur() {
    let mut e = std_engine();
    e.process_string("ddawks", VIE);
    assert_eq!(e.processed_string(VIE), "đắk");

    e.reset();
    e.process_string("tooi oo HIEEUR", VIE);
    assert_eq!(e.processed_string(VIE), "HIỂU");

    e.reset();
    e.process_string("NGUOIW", VIE);
    assert_eq!(e.processed_string(VIE), "NGƯƠI");

    e.reset();
    e.process_string("{s", VIE);
    assert_eq!(e.processed_string(VIE), "Ớ");
}

#[test]
fn vni_double_tone_key() {
    let mut e = engine("VNI", EngineFlags::STD);
    e.process_string("o55", VIE);
    assert_eq!(e.processed_string(VIE), "o5");
}

#[test]
fn duwongwj_fallback() {
    let mut e = std_engine();
    e.process_string("duwongwj", VIE);
    assert_eq!(e.processed_string(VIE), "duongwj");
}

#[test]
fn free_tone_style_words() {
    // Telex 2 with free tone marking but not the standard tone-placement style.
    let flags = EngineFlags::STD.difference(EngineFlags::STD_TONE_STYLE);
    let mut e = engine("Telex 2", flags);
    e.process_string("choas", VIE);
    assert_eq!(e.processed_string(VIE), "choá");
    e.reset();
    e.process_string("bieecs", VIE);
    assert_eq!(e.processed_string(VIE), "biếc");
    e.reset();
    e.process_string("uese", VIE);
    assert_eq!(e.processed_string(VIE), "uế");
}

#[test]
fn restore_last_word() {
    let mut e = std_engine();
    e.process_string("duwongj tooi", VIE);
    e.restore_last_word(false);
    assert_eq!(e.processed_string(VIE), "tooi");
}

#[test]
fn restore_last_word_microsoft() {
    let mut e = engine("Microsoft layout", EngineFlags::STD);
    e.process_string("112", VIE);
    assert_eq!(e.processed_string(VIE), "1â");
    e.restore_last_word(false);
    assert_eq!(e.processed_string(ENG), "12");

    e.reset();
    e.process_string("d[]ng9 t4i", VIE);
    e.restore_last_word(false);
    assert_eq!(e.processed_string(VIE), "t4i");
}

#[test]
fn z_processing() {
    let mut e = std_engine();
    e.process_string("loz", VIE);
    assert_eq!(e.processed_string(VIE), "loz");

    e.reset();
    e.process_string("losz", VIE);
    assert_eq!(e.processed_string(VIE), "lo");
    assert_eq!(e.processed_string(ENG), "losz");
}

#[test]
fn vn_word_tone_overrides() {
    let mut e = std_engine();
    e.process_string("tôifs", VIE);
    assert_eq!(e.processed_string(VIE), "tối");
    assert_eq!(e.processed_string(ENG), "tôifs");

    e.reset();
    e.process_string("tốif", VIE);
    assert_eq!(e.processed_string(VIE), "tồi");
    assert_eq!(e.processed_string(ENG), "tốif");

    e.reset();
    e.process_string("tốiz", VIE);
    assert_eq!(e.processed_string(VIE), "tôi");
}

#[test]
fn double_typing_corpus() {
    let mut e = std_engine();
    e.process_string("linux", VIE);
    e.process_string("x", VIE);
    assert_eq!(e.processed_string(VIE), "linux");

    e.reset();
    e.process_string("buwo", VIE);
    e.process_string("o", VIE);
    assert_eq!(e.processed_string(VIE), "buô");

    e.reset();
    e.process_string("buowc", VIE);
    e.process_string("o", VIE);
    assert_eq!(e.processed_string(VIE), "buôc");

    e.reset();
    e.process_string("cuoiw", VIE);
    e.process_string("o", VIE);
    assert_eq!(e.processed_string(VIE), "cuôi");

    e.reset();
    e.process_string("ach", VIE);
    e.process_string("a", VIE);
    assert_eq!(e.processed_string(VIE), "acha");

    e.reset();
    e.process_string("nhuw", VIE);
    assert_eq!(e.processed_string(VIE), "như");
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("thuw", VIE);
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("thow", VIE);
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("tooi", VIE);
    assert_eq!(e.processed_string(VIE), "tôi");
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("arch", VIE);
    assert!(!e.is_valid(false));

    e.reset();
    e.process_string("[[", VIE);
    e.process_string("oo", VIE);
    assert_eq!(e.processed_string(VIE), "[ô");

    e.reset();
    e.process_string("oo]", VIE);
    assert_eq!(e.processed_string(VIE), "ôư");

    e.reset();
    e.process_string("chury", VIE);
    assert!(e.is_valid(true));

    e.reset();
    e.process_string("turyn", VIE);
    e.remove_last_char(true);
    e.remove_last_char(true);
    assert_eq!(e.processed_string(VIE), "tủ");

    e.reset();
    e.process_string("chuyển", VIE);
    e.process_string("z", VIE);
    assert_eq!(e.processed_string(VIE), "chuyên");

    e.reset();
    e.process_string("nhueej", VIE);
    assert_eq!(e.processed_string(VIE), "nhuệ");

    e.reset();
    e.process_string("cuongw", VIE);
    assert_eq!(e.processed_string(VIE), "cương");

    e.reset();
    e.process_string("quawcj", VIE);
    assert_eq!(e.processed_string(VIE), "quặc");

    e.reset();
    e.process_string("tôi）t", ENG);
    assert_eq!(e.processed_string(VIE), "t");
}

#[test]
fn vni_afnor_triggers() {
    let mut e = engine("VNI (AZERTY, AFNOR)", EngineFlags::STD);

    // Accented-letter triggers map to their own diacritic.
    for (keys, want) in [("aé", "á"), ("aè", "à"), ("aê", "â")] {
        e.reset();
        e.process_string(keys, VIE);
        assert_eq!(e.processed_string(VIE), want, "{keys}");
    }

    // Punctuation triggers relocated for AFNOR's unshifted layer.
    for (keys, want) in [
        ("a)", "ả"), // hook
        ("a(", "ã"), // tilde
        ("a-", "ạ"), // dot
        ("u«", "ư"), // horn
        ("a»", "ă"), // breve
        ("dà", "đ"), // đ
    ] {
        e.reset();
        e.process_string(keys, VIE);
        assert_eq!(e.processed_string(VIE), want, "{keys}");
    }

    // A full syllable: viet + ê (circumflex) + é (acute) -> viết.
    e.reset();
    e.process_string("vietêé", VIE);
    assert_eq!(e.processed_string(VIE), "viết");
}
