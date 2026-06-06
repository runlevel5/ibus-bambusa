//! The ibus-bambusa engine binary: connects to IBus and serves a factory whose
//! engines compose Vietnamese in preedit mode.

mod config;
mod flags;
mod keysyms;
mod preedit;

use config::Config;
use ibus_zbus::{EngineHandler, Factory, address, consts};
use preedit::PreeditHandler;

const BUS_NAME: &str = "org.freedesktop.IBus.bambusa";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--version") {
        println!(env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = Config::load();

    let connection = address::connect().await?;
    connection.request_name(BUS_NAME).await?;

    let factory = Factory::new(move |_engine_name| {
        Box::new(PreeditHandler::new(config.clone())) as Box<dyn EngineHandler>
    });
    connection
        .object_server()
        .at(consts::FACTORY_PATH, factory)
        .await?;

    eprintln!("ibus-bambusa: registered {BUS_NAME}, serving factory");
    std::future::pending::<()>().await;
    Ok(())
}
