//! The `org.freedesktop.IBus.Factory` interface.
//!
//! IBus calls `CreateEngine` once per input context. Each call builds a fresh
//! handler, serves a new [`EngineInterface`] at a unique object path, and
//! spawns its actor.

use std::sync::atomic::{AtomicU64, Ordering};

use zbus::object_server::SignalEmitter;
use zbus::{Connection, ObjectServer, fdo, interface};
use zvariant::OwnedObjectPath;

use crate::engine::EngineInterface;
use crate::handler::EngineHandler;

type HandlerBuilder = Box<dyn Fn(&str) -> Box<dyn EngineHandler> + Send + Sync>;

/// Creates engine instances on demand, each with its own handler.
pub struct Factory {
    new_handler: HandlerBuilder,
    counter: AtomicU64,
}

impl Factory {
    /// Build a factory that calls `new_handler(engine_name)` per input context.
    pub fn new(
        new_handler: impl Fn(&str) -> Box<dyn EngineHandler> + Send + Sync + 'static,
    ) -> Self {
        Self {
            new_handler: Box::new(new_handler),
            counter: AtomicU64::new(0),
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    async fn create_engine(
        &self,
        engine_name: &str,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(connection)] conn: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let path = OwnedObjectPath::try_from(format!("/org/freedesktop/IBus/Engine/bambusa/{n}"))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let handler = (self.new_handler)(engine_name);
        let emitter =
            SignalEmitter::new(conn, &path).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let (iface, actor) = EngineInterface::new(handler, emitter.to_owned());
        tokio::spawn(actor);
        server.at(&path, iface).await?;
        Ok(path)
    }

    async fn destroy(&self) {}
}
