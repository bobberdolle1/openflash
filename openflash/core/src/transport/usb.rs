//! USB transport.
//!
//! Enumerates OpenFlash devices by vendor/product id and exchanges protocol
//! frames over the bulk endpoint pair the firmware exposes. Blocking
//! throughout: `nusb` provides blocking transfer waits, so the host tools need
//! no async runtime to talk to hardware.

use std::time::Duration;

use nusb::transfer::{Buffer, Bulk, In, Out};
use nusb::{Device, Endpoint, MaybeFuture};

use openflash_protocol::frame::{Frame, FrameError, HEADER_LEN, MAX_FRAME};
use openflash_protocol::{USB_ENDPOINT_IN, USB_ENDPOINT_OUT, USB_PRODUCT_ID, USB_VENDOR_ID};

use super::{Transport, TransportError, TransportKind, TransportResult};

/// An OpenFlash device found on the USB bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDeviceInfo {
    /// Stable-ish address of the form `bus-address`.
    pub address: String,
    /// Serial number the device reports, when it has one.
    pub serial_number: Option<String>,
    /// Product string the device reports.
    pub product: Option<String>,
}

impl UsbDeviceInfo {
    /// Identifier accepted by `--device`.
    pub fn selector(&self) -> String {
        self.serial_number
            .clone()
            .unwrap_or_else(|| self.address.clone())
    }
}

/// List every connected OpenFlash device.
///
/// Returns an empty list when none are attached; that is not an error, and the
/// caller is expected to say so rather than to invent a device.
pub fn list_devices() -> TransportResult<Vec<UsbDeviceInfo>> {
    let devices = nusb::list_devices().wait().map_err(|e| {
        TransportError::Io(std::io::Error::other(format!(
            "cannot list USB devices: {e}"
        )))
    })?;

    Ok(devices
        .filter(|info| info.vendor_id() == USB_VENDOR_ID && info.product_id() == USB_PRODUCT_ID)
        .map(|info| UsbDeviceInfo {
            address: format!("{}-{}", info.bus_id(), info.device_address()),
            serial_number: info.serial_number().map(str::to_string),
            product: info.product_string().map(str::to_string),
        })
        .collect())
}

/// A device reached over USB.
pub struct UsbTransport {
    address: String,
    endpoint_out: Endpoint<Bulk, Out>,
    endpoint_in: Endpoint<Bulk, In>,
    // Kept alive: dropping the device would invalidate the endpoints.
    _device: Device,
}

impl UsbTransport {
    /// Open the only connected device.
    ///
    /// Fails when none is attached, and when more than one is — picking
    /// arbitrarily could send a chip-erase to the wrong board.
    pub fn open_only() -> TransportResult<Self> {
        let mut found = list_devices()?;
        match found.len() {
            0 => Err(TransportError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "no OpenFlash device found (looking for USB {USB_VENDOR_ID:#06x}:{USB_PRODUCT_ID:#06x})"
                ),
            ))),
            1 => Self::open(&found.remove(0).selector()),
            n => Err(TransportError::Io(std::io::Error::other(format!(
                "{n} OpenFlash devices are connected; select one with --device {}",
                found
                    .iter()
                    .map(|d| d.selector())
                    .collect::<Vec<_>>()
                    .join(" | ")
            )))),
        }
    }

    /// Open a device by serial number or `bus-address`.
    pub fn open(selector: &str) -> TransportResult<Self> {
        let devices = nusb::list_devices().wait().map_err(|e| {
            TransportError::Io(std::io::Error::other(format!(
                "cannot list USB devices: {e}"
            )))
        })?;

        let target = devices
            .filter(|info| info.vendor_id() == USB_VENDOR_ID && info.product_id() == USB_PRODUCT_ID)
            .find(|info| {
                let address = format!("{}-{}", info.bus_id(), info.device_address());
                address == selector || info.serial_number() == Some(selector)
            })
            .ok_or_else(|| {
                TransportError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no OpenFlash device matches '{selector}'"),
                ))
            })?;

        let address = format!("{}-{}", target.bus_id(), target.device_address());
        let device = target.open().wait().map_err(|e| {
            TransportError::Io(std::io::Error::other(format!(
                "cannot open USB device {address}: {e} \
                 (on Linux this is usually a missing udev rule; see docs/HARDWARE_GUIDE.md)"
            )))
        })?;

        let interface = device.claim_interface(0).wait().map_err(|e| {
            TransportError::Io(std::io::Error::other(format!(
                "cannot claim interface 0 on {address}: {e} \
                 (another process may already have the device open)"
            )))
        })?;

        let endpoint_out = interface
            .endpoint::<Bulk, Out>(USB_ENDPOINT_OUT)
            .map_err(|e| {
                TransportError::Io(std::io::Error::other(format!(
                    "device {address} has no bulk OUT endpoint {USB_ENDPOINT_OUT:#04x}: {e}"
                )))
            })?;
        let endpoint_in = interface
            .endpoint::<Bulk, In>(USB_ENDPOINT_IN)
            .map_err(|e| {
                TransportError::Io(std::io::Error::other(format!(
                    "device {address} has no bulk IN endpoint {USB_ENDPOINT_IN:#04x}: {e}"
                )))
            })?;

        Ok(Self {
            address,
            endpoint_out,
            endpoint_in,
            _device: device,
        })
    }

    /// Bus address of the open device.
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl Transport for UsbTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Usb {
            address: self.address.clone(),
        }
    }

    fn exchange(&mut self, request: &[u8], timeout: Duration) -> TransportResult<Vec<u8>> {
        self.endpoint_out
            .transfer_blocking(Buffer::from(request.to_vec()), timeout)
            .into_result()
            .map_err(|e| {
                TransportError::Io(std::io::Error::other(format!("USB write failed: {e}")))
            })?;

        // A frame can span several bulk packets, so keep reading until one has
        // fully arrived rather than assuming a single transfer holds it.
        let packet = self.endpoint_in.max_packet_size().max(64);
        let mut buffer = Vec::with_capacity(HEADER_LEN + packet);

        loop {
            match Frame::decode(&buffer) {
                Ok(frame) => {
                    buffer.truncate(frame.encoded_len());
                    return Ok(buffer);
                }
                Err(FrameError::Incomplete { .. }) => {}
                Err(other) => return Err(other.into()),
            }

            let completion = self
                .endpoint_in
                .transfer_blocking(Buffer::new(packet), timeout);
            let read = completion.actual_len;
            completion.status.map_err(|e| {
                TransportError::Io(std::io::Error::other(format!("USB read failed: {e}")))
            })?;

            if read == 0 {
                return Err(TransportError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "device returned a zero-length packet after {} bytes, mid-frame",
                        buffer.len()
                    ),
                )));
            }
            buffer.extend_from_slice(&completion.buffer[..read]);

            if buffer.len() > MAX_FRAME {
                return Err(TransportError::Frame(FrameError::PayloadTooLarge {
                    len: buffer.len(),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enumeration must be able to run on a machine with no device attached and
    /// report that plainly, rather than failing or inventing one. Runs on CI,
    /// where there is never an OpenFlash device.
    #[test]
    fn listing_devices_succeeds_even_with_none_attached() {
        match list_devices() {
            Ok(devices) => {
                for device in devices {
                    assert!(!device.selector().is_empty());
                }
            }
            // A CI container may have no USB subsystem at all; that is a
            // reportable I/O error, not a panic.
            Err(TransportError::Io(_)) => {}
            Err(other) => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn opening_a_nonexistent_selector_fails_with_not_found() {
        match UsbTransport::open("definitely-not-a-real-device") {
            Err(TransportError::Io(e)) => assert!(
                e.kind() == std::io::ErrorKind::NotFound || e.kind() == std::io::ErrorKind::Other,
                "unexpected io error kind: {:?}",
                e.kind()
            ),
            Err(other) => panic!("unexpected error: {other:?}"),
            Ok(_) => panic!("a bogus selector must not open a device"),
        }
    }

    #[test]
    fn a_selector_prefers_the_serial_number() {
        let with_serial = UsbDeviceInfo {
            address: "1-4".into(),
            serial_number: Some("OF-0001".into()),
            product: None,
        };
        assert_eq!(with_serial.selector(), "OF-0001");

        let without_serial = UsbDeviceInfo {
            address: "1-4".into(),
            serial_number: None,
            product: None,
        };
        assert_eq!(without_serial.selector(), "1-4");
    }
}
