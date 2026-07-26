//! Turning command-line flags into a connected device.
//!
//! A command that touches hardware either gets a real device here or fails.
//! There is no fallback that fabricates one, which is what the previous
//! implementation did: it "connected" unconditionally and then reported
//! plausible-looking results for chips that were never attached.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use openflash_core::device::Device;
use openflash_core::emulator::EmulatedDevice;
#[cfg(unix)]
use openflash_core::transport::UnixTransport;
#[cfg(feature = "usb")]
use openflash_core::transport::{list_devices, UsbTransport};
use openflash_core::transport::{TcpTransport, Transport};

/// A device connection, whatever it is reached over.
pub type Connection = Device<Box<dyn Transport>>;

/// How the user asked to reach a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Use the single attached USB device.
    AutoUsb,
    /// A USB device by serial number or `bus-address`.
    Usb(String),
    /// An SBC agent at `host:port`.
    Tcp(String),
    /// An SBC agent on a Unix socket.
    Unix(String),
    /// The in-process emulator, backed by a chip of this many bytes and
    /// optionally by a file that keeps its contents between runs.
    Emulator {
        /// Chip size in bytes.
        size: usize,
        /// File the contents are loaded from and saved to.
        image: Option<PathBuf>,
    },
}

impl Target {
    /// Work out the target from the global flags.
    ///
    /// The flags are mutually exclusive: pointing at two devices at once is a
    /// mistake worth reporting rather than resolving by precedence.
    pub fn from_flags(
        device: Option<&str>,
        tcp: Option<&str>,
        unix: Option<&str>,
        emulate: Option<u64>,
        emulate_image: Option<&Path>,
    ) -> Result<Self> {
        let chosen = [
            device.map(|_| "--device"),
            tcp.map(|_| "--tcp"),
            unix.map(|_| "--unix"),
            emulate.map(|_| "--emulate"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if chosen.len() > 1 {
            bail!(
                "{} are mutually exclusive: pick one device to talk to",
                chosen.join(", ")
            );
        }

        Ok(match (device, tcp, unix, emulate) {
            (Some(selector), ..) => Self::Usb(selector.to_string()),
            (_, Some(endpoint), ..) => Self::Tcp(endpoint.to_string()),
            (_, _, Some(path), _) => Self::Unix(path.to_string()),
            (.., Some(size)) => Self::Emulator {
                size: usize::try_from(size).with_context(|| {
                    format!("emulated chip size {size} does not fit in memory on this platform")
                })?,
                image: emulate_image.map(Path::to_path_buf),
            },
            _ => Self::AutoUsb,
        })
    }

    /// Whether this target is emulated rather than real hardware.
    pub fn is_emulated(&self) -> bool {
        matches!(self, Self::Emulator { .. })
    }
}

/// Open a connection to the target, performing the protocol handshake.
pub fn open(target: &Target, timeout: Duration) -> Result<Connection> {
    let transport: Box<dyn Transport> = match target {
        Target::Emulator { size, image } => {
            let sector = openflash_core::emulator::SECTOR_SIZE;
            if *size < sector || !size.is_power_of_two() {
                bail!(
                    "--emulate takes a power of two of at least {sector} bytes (one erase \
                     sector); real SPI NOR capacities are powers of two, and the JEDEC id \
                     the emulator reports is derived from the size"
                );
            }
            match image {
                Some(path) => Box::new(
                    EmulatedDevice::with_backing_file("EMULATED", [0xEF, 0x40, 0x15], path, *size)
                        .with_context(|| {
                            format!("cannot use {} as an emulated chip image", path.display())
                        })?,
                ),
                None => Box::new(EmulatedDevice::new("EMULATED", [0xEF, 0x40, 0x15], *size)),
            }
        }
        Target::Tcp(endpoint) => Box::new(
            TcpTransport::connect(endpoint, timeout)
                .with_context(|| format!("cannot reach an OpenFlash agent at {endpoint}"))?,
        ),
        #[cfg(unix)]
        Target::Unix(path) => Box::new(
            UnixTransport::connect(path)
                .with_context(|| format!("cannot reach an OpenFlash agent on {path}"))?,
        ),
        #[cfg(not(unix))]
        Target::Unix(_) => bail!("Unix sockets are not available on this platform"),
        #[cfg(feature = "usb")]
        Target::AutoUsb => Box::new(UsbTransport::open_only()?),
        #[cfg(feature = "usb")]
        Target::Usb(selector) => Box::new(UsbTransport::open(selector)?),
        #[cfg(not(feature = "usb"))]
        Target::AutoUsb | Target::Usb(_) => {
            bail!("this build was compiled without USB support; use --tcp or --unix")
        }
    };

    let device = Device::connect(transport).context(
        "the device did not complete the protocol handshake; \
         check that its firmware is built from this revision",
    )?;
    Ok(device)
}

/// The banner printed whenever a command runs against the emulator.
///
/// Emulated runs must never be mistakable for a real one, so this goes to
/// stderr — it survives `--format json` and output redirection.
pub fn warn_if_emulated(target: &Target, quiet: bool) {
    if target.is_emulated() && !quiet {
        eprintln!(
            "{} running against the in-process emulator: no hardware is involved \
             and no real chip is read or written",
            "EMULATED:".yellow().bold()
        );
    }
}

/// List attached USB devices.
#[cfg(feature = "usb")]
pub fn scan() -> Result<Vec<String>> {
    Ok(list_devices()?
        .into_iter()
        .map(|device| {
            let product = device.product.unwrap_or_else(|| "OpenFlash".to_string());
            match device.serial_number {
                Some(serial) => format!("{} at {} (serial {serial})", product, device.address),
                None => format!("{} at {}", product, device.address),
            }
        })
        .collect())
}

/// List attached USB devices.
#[cfg(not(feature = "usb"))]
pub fn scan() -> Result<Vec<String>> {
    bail!("this build was compiled without USB support")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emulator(size: usize) -> Target {
        Target::Emulator { size, image: None }
    }

    #[test]
    fn no_flags_means_the_single_usb_device() {
        assert_eq!(
            Target::from_flags(None, None, None, None, None).unwrap(),
            Target::AutoUsb
        );
    }

    #[test]
    fn each_flag_selects_its_transport() {
        assert_eq!(
            Target::from_flags(Some("OF-1"), None, None, None, None).unwrap(),
            Target::Usb("OF-1".into())
        );
        assert_eq!(
            Target::from_flags(None, Some("pi:9999"), None, None, None).unwrap(),
            Target::Tcp("pi:9999".into())
        );
        assert_eq!(
            Target::from_flags(None, None, Some("/run/of.sock"), None, None).unwrap(),
            Target::Unix("/run/of.sock".into())
        );
        assert_eq!(
            Target::from_flags(None, None, None, Some(65536), None).unwrap(),
            emulator(65536)
        );
    }

    /// Pointing at two devices is a mistake, and silently preferring one of
    /// them could send a write to the wrong place.
    #[test]
    fn conflicting_flags_are_rejected() {
        let error = Target::from_flags(Some("OF-1"), Some("pi:9999"), None, None, None)
            .expect_err("two targets must not resolve");
        let message = error.to_string();
        assert!(message.contains("--device"), "{message}");
        assert!(message.contains("--tcp"), "{message}");
    }

    #[test]
    fn only_the_emulator_counts_as_emulated() {
        assert!(emulator(4096).is_emulated());
        assert!(!Target::AutoUsb.is_emulated());
        assert!(!Target::Tcp("x:1".into()).is_emulated());
    }

    #[test]
    fn an_emulated_size_that_is_not_a_power_of_two_is_rejected() {
        let error = open(&emulator(5000), Duration::from_secs(1))
            .expect_err("5000 bytes is not a power of two");
        assert!(error.to_string().contains("power of two"), "{error}");
    }

    #[test]
    fn an_emulated_size_below_one_sector_is_rejected() {
        let error =
            open(&emulator(512), Duration::from_secs(1)).expect_err("512 bytes is under a sector");
        assert!(error.to_string().contains("4096"), "{error}");
    }

    #[test]
    fn opening_the_emulator_completes_the_handshake() {
        let device = open(&emulator(64 * 1024), Duration::from_secs(1)).unwrap();
        assert_eq!(
            device.version().protocol,
            openflash_core::protocol::PROTOCOL_VERSION
        );
    }

    /// A file-backed emulated chip must keep what was written to it, otherwise
    /// two consecutive CLI invocations talk to two different blank chips.
    #[test]
    fn a_file_backed_emulated_chip_keeps_its_contents() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("chip.bin");
        let target = Target::Emulator {
            size: 64 * 1024,
            image: Some(path.clone()),
        };

        let payload = vec![0x5Au8; 300];
        {
            let mut device = open(&target, Duration::from_secs(1)).unwrap();
            device
                .program(
                    0,
                    &payload,
                    openflash_core::device::ProgramOptions::default(),
                    None,
                )
                .unwrap();
        } // dropped here, so the image is written back

        let mut device = open(&target, Duration::from_secs(1)).unwrap();
        assert_eq!(
            device.read_to_vec(0, payload.len() as u64).unwrap(),
            payload
        );
    }
}
