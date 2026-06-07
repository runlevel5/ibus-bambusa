//! The ibus-bambusa engine binary: connects to IBus and serves a factory whose
//! engines compose Vietnamese in preedit mode.

mod dict;
mod engines;
mod keysyms;
mod macros;
mod preedit;

use bambusa_config::Config;
use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use ibus_zbus::{EngineHandler, Factory, address, consts};
use preedit::PreeditHandler;

const BUS_NAME: &str = "org.freedesktop.IBus.bambusa";
const GETTEXT_DOMAIN: &str = "ibus-bambusa";
const LOCALEDIR: &str = "/usr/share/locale";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--version") {
        println!(env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    setlocale(LocaleCategory::LcAll, "");
    let _ = bindtextdomain(GETTEXT_DOMAIN, LOCALEDIR);
    let _ = bind_textdomain_codeset(GETTEXT_DOMAIN, "UTF-8");
    let _ = textdomain(GETTEXT_DOMAIN);

    let config = Config::load();

    let connection = address::connect().await?;
    connection.request_name(BUS_NAME).await?;

    let factory = Factory::new(move |engine_name| {
        let mut cfg = config.clone();
        cfg.input_method = engines::method_for_engine(engine_name).to_string();
        Box::new(PreeditHandler::new(cfg)) as Box<dyn EngineHandler>
    });
    connection
        .object_server()
        .at(consts::FACTORY_PATH, factory)
        .await?;

    eprintln!("ibus-bambusa: registered {BUS_NAME}, serving factory");
    std::future::pending::<()>().await;
    Ok(())
}
