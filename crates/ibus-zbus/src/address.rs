//! Discovering and connecting to the IBus private bus.
//!
//! IBus runs its own D-Bus daemon, separate from the session bus. Its address
//! is published in `IBUS_ADDRESS`, or in a per-display file under
//! `~/.config/ibus/bus/`. On Wayland the file is keyed by the Wayland display.

use std::fmt;
use std::path::PathBuf;
use std::{env, fs, io};

use zbus::{AuthMechanism, Connection, connection};

/// Failure to locate or connect to the IBus bus.
#[derive(Debug)]
pub enum Error {
    /// The bus address could not be discovered.
    Discovery(io::Error),
    /// Connecting to the discovered address failed.
    Connection(zbus::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Discovery(e) => write!(f, "could not find the IBus address: {e}"),
            Error::Connection(e) => write!(f, "could not connect to IBus: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Discovery(e) => Some(e),
            Error::Connection(e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Discovery(e)
    }
}

impl From<zbus::Error> for Error {
    fn from(e: zbus::Error) -> Self {
        Error::Connection(e)
    }
}

/// The IBus bus-address file name for a machine id and Wayland display.
fn bus_file_name(machine_id: &str, display: &str) -> String {
    format!("{machine_id}-unix-{display}")
}

/// Extract the address from the contents of an IBus bus-address file.
fn parse_address_file(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("IBUS_ADDRESS=").map(str::to_string))
}

fn config_dir() -> PathBuf {
    match env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config"),
    }
}

fn machine_id() -> io::Result<String> {
    for path in ["/var/lib/dbus/machine-id", "/etc/machine-id"] {
        if let Ok(id) = fs::read_to_string(path) {
            return Ok(id.trim().to_string());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no machine-id under /var/lib/dbus or /etc",
    ))
}

/// Path to the IBus bus-address file for the current Wayland session.
fn socket_path() -> io::Result<PathBuf> {
    if let Ok(path) = env::var("IBUS_ADDRESS_FILE")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let display = env::var("WAYLAND_DISPLAY").unwrap_or_default();
    if display.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "WAYLAND_DISPLAY is not set; a Wayland session is required",
        ));
    }
    Ok(config_dir()
        .join("ibus")
        .join("bus")
        .join(bus_file_name(&machine_id()?, &display)))
}

/// Discover the IBus private-bus address.
pub fn ibus_address() -> io::Result<String> {
    if let Ok(addr) = env::var("IBUS_ADDRESS")
        && !addr.is_empty()
    {
        return Ok(addr);
    }
    let path = socket_path()?;
    let contents = fs::read_to_string(&path)?;
    parse_address_file(&contents).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no IBUS_ADDRESS line in {}", path.display()),
        )
    })
}

/// Connect to the IBus private bus using EXTERNAL authentication.
pub async fn connect() -> Result<Connection, Error> {
    let address = ibus_address()?;
    let connection = connection::Builder::address(address.as_str())?
        .auth_mechanism(AuthMechanism::External)
        .build()
        .await?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_file_name_format() {
        assert_eq!(
            bus_file_name("abc123", "wayland-0"),
            "abc123-unix-wayland-0"
        );
    }

    #[test]
    fn parse_address_from_file() {
        let contents =
            "# some comment\nIBUS_ADDRESS=unix:abstract=/tmp/dbus-x,guid=ab\nIBUS_DAEMON_PID=42\n";
        assert_eq!(
            parse_address_file(contents).as_deref(),
            Some("unix:abstract=/tmp/dbus-x,guid=ab")
        );
        assert_eq!(parse_address_file("no address here\n"), None);
    }
}
