//! A minimal IBus engine that echoes the typed ASCII character.
//!
//! Run inside a session with the IBus daemon available. It registers the
//! `org.freedesktop.IBus.bambusa` bus name and serves a factory whose engines
//! commit each typed character straight back — enough to prove the binding
//! talks to IBus end to end.

use ibus_zbus::{EchoHandler, EngineHandler, Factory, address, consts};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = address::connect().await?;
    connection
        .request_name("org.freedesktop.IBus.bambusa")
        .await?;

    let factory = Factory::new(|_engine_name| Box::new(EchoHandler) as Box<dyn EngineHandler>);
    connection
        .object_server()
        .at(consts::FACTORY_PATH, factory)
        .await?;

    println!("echo engine registered; serving the IBus factory…");
    std::future::pending::<()>().await;
    Ok(())
}
