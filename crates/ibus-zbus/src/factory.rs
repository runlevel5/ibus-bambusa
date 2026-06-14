//! The `org.freedesktop.IBus.Factory` interface.
//!
//! IBus calls `CreateEngine` once per input context. Each call builds a fresh
//! handler, serves a new [`EngineInterface`] at a unique object path, and
//! spawns its actor.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use zbus::object_server::SignalEmitter;
use zbus::{Connection, ObjectServer, fdo, interface};
use zvariant::OwnedObjectPath;

use crate::engine::EngineInterface;
use crate::handler::EngineHandler;

type HandlerBuilder = Box<dyn Fn(&str) -> Box<dyn EngineHandler> + Send + Sync>;

/// Upper bound on concurrently live engines. Normal use needs ~1 (IBus creates
/// one per input context and destroys it with the context); this is a generous
/// backstop so a hostile same-user caller can't exhaust tasks and object-server
/// registrations by looping `CreateEngine` without `Destroy`.
const MAX_LIVE_ENGINES: usize = 64;

/// Releases one live-engine slot when an engine actor ends. Held by the actor
/// (see `EngineInterface::new`), so it drops on clean destroy, channel close, or
/// actor panic alike.
struct LiveGuard(Arc<AtomicUsize>);

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Creates engine instances on demand, each with its own handler.
pub struct Factory {
    new_handler: HandlerBuilder,
    counter: AtomicU64,
    /// Number of engines currently alive, for the `MAX_LIVE_ENGINES` cap.
    live: Arc<AtomicUsize>,
}

impl Factory {
    /// Build a factory that calls `new_handler(engine_name)` per input context.
    pub fn new(
        new_handler: impl Fn(&str) -> Box<dyn EngineHandler> + Send + Sync + 'static,
    ) -> Self {
        Self {
            new_handler: Box::new(new_handler),
            counter: AtomicU64::new(0),
            live: Arc::new(AtomicUsize::new(0)),
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
        // Reserve a live-engine slot up front; refuse past the cap so a flood of
        // CreateEngine calls can't exhaust tasks and registrations.
        if self.live.fetch_add(1, Ordering::AcqRel) >= MAX_LIVE_ENGINES {
            self.live.fetch_sub(1, Ordering::AcqRel);
            return Err(fdo::Error::LimitsExceeded(
                "too many live bambusa engines".into(),
            ));
        }
        // From here `guard` owns the slot: any early return below drops it and
        // frees the slot; on success it moves into the engine actor, which drops
        // it when the engine is destroyed.
        let guard = LiveGuard(self.live.clone());

        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let path = OwnedObjectPath::try_from(format!("/org/freedesktop/IBus/Engine/bambusa/{n}"))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let handler = (self.new_handler)(engine_name);
        let emitter =
            SignalEmitter::new(conn, &path).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let (iface, actor) = EngineInterface::new(
            handler,
            emitter.to_owned(),
            conn.clone(),
            path.clone(),
            Box::new(guard),
        );
        tokio::spawn(actor);
        server.at(&path, iface).await?;
        Ok(path)
    }

    async fn destroy(&self) {}
}
