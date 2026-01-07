//! Hardware Expansion Module - v2.1
//! 
//! Official OpenFlash PCB, adapters, logic analyzer mode, and debug interfaces.

use serde::{Deserialize, Serialize};

// ============================================================================
// Error Types
// ============================================================================

/// Hardware module errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareError {
    /// PCB not detected
    PcbNotDetected,
    /// Invalid PCB revision
    InvalidPcbRevision(u8),
    /// Adapter not connected
    AdapterNotConnected,
    /// Socket type mismatch
    SocketMismatch { expected: SocketType, found: SocketType },
    /// Logic analyzer buffer overflow
    LogicAnalyzerOverflow,
    /// Capture timeout
    CaptureTimeout,
    /// JTAG chain error
    JtagChainError(String),
    /// SWD communication error
    SwdError(String),
    /// OLED display error
    DisplayError(String),
    /// I2C communication error
    I2cError(String),
    /// Configuration error
    ConfigError(String),
}

pub type HardwareResult<T> = Result<T, HardwareError>;


// ============================================================================
// OpenFlash PCB v1
// ============================================================================

/// PCB revision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcbRevision {
    /// Rev A - Initial release
    RevA,
    /// Rev B - Bug fixes
    RevB,
    /// Rev C - Production
    RevC,
}

impl PcbRevision {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::RevA),
            0x02 => Some(Self::RevB),
            0x03 => Some(Self::RevC),
            _ => None,
        }
    }
}

/// Socket type on the PCB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketType {
    /// No socket / empty
    None,
    /// TSOP-48 ZIF socket for parallel NAND
    Tsop48,
    /// SOP-8 socket for SPI NAND/NOR
    Sop8,
    /// BGA socket adapter
    Bga,
    /// eMMC socket (BGA-153/169)
    Emmc,
    /// SD card slot
    SdCard,
}

/// OpenFlash PCB v1 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFlashPcb {
    /// PCB revision
    pub revision: PcbRevision,
    /// Serial number
    pub serial_number: String,
    /// Firmware version on RP2040
    pub rp2040_firmware: String,
    /// Firmware version on ESP32
    pub esp32_firmware: Option<String>,
    /// Currently installed socket
    pub active_socket: SocketType,
    /// OLED display present
    pub has_oled: bool,
    /// WiFi enabled (ESP32)
    pub wifi_enabled: bool,
    /// USB-C connected
    pub usb_connected: bool,
}


impl Default for OpenFlashPcb {
    fn default() -> Self {
        Self {
            revision: PcbRevision::RevA,
            serial_number: String::new(),
            rp2040_firmware: String::from("2.1.0"),
            esp32_firmware: None,
            active_socket: SocketType::None,
            has_oled: false,
            wifi_enabled: false,
            usb_connected: false,
        }
    }
}

impl OpenFlashPcb {
    /// Detect PCB from device info
    pub fn detect() -> HardwareResult<Self> {
        // In real implementation, query device via USB
        Ok(Self::default())
    }

    /// Get PCB capabilities
    pub fn capabilities(&self) -> PcbCapabilities {
        PcbCapabilities {
            parallel_nand: true,
            spi_nand: true,
            spi_nor: true,
            emmc: true,
            ufs: self.revision != PcbRevision::RevA,
            logic_analyzer: true,
            jtag: true,
            swd: true,
            wifi: self.esp32_firmware.is_some(),
            oled: self.has_oled,
        }
    }
}

/// PCB capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbCapabilities {
    pub parallel_nand: bool,
    pub spi_nand: bool,
    pub spi_nor: bool,
    pub emmc: bool,
    pub ufs: bool,
    pub logic_analyzer: bool,
    pub jtag: bool,
    pub swd: bool,
    pub wifi: bool,
    pub oled: bool,
}


// ============================================================================
// TSOP-48 ZIF Adapter
// ============================================================================

/// TSOP-48 pin configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tsop48Pinout {
    /// Standard NAND pinout (most common)
    StandardNand,
    /// Samsung specific pinout
    Samsung,
    /// Hynix specific pinout
    Hynix,
    /// Micron specific pinout
    Micron,
    /// Toshiba specific pinout
    Toshiba,
}

/// TSOP-48 ZIF adapter board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tsop48Adapter {
    /// Adapter revision
    pub revision: u8,
    /// Current pinout configuration
    pub pinout: Tsop48Pinout,
    /// Chip inserted
    pub chip_detected: bool,
    /// Voltage level (3.3V or 1.8V)
    pub voltage: VoltageLevel,
    /// Bus width (8 or 16 bit)
    pub bus_width: BusWidth,
}

/// Voltage level for flash chips
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoltageLevel {
    V3_3,
    V1_8,
}

/// Bus width
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusWidth {
    X8,
    X16,
}

impl Default for Tsop48Adapter {
    fn default() -> Self {
        Self {
            revision: 1,
            pinout: Tsop48Pinout::StandardNand,
            chip_detected: false,
            voltage: VoltageLevel::V3_3,
            bus_width: BusWidth::X8,
        }
    }
}

impl Tsop48Adapter {
    /// Set pinout configuration
    pub fn set_pinout(&mut self, pinout: Tsop48Pinout) {
        self.pinout = pinout;
    }

    /// Set voltage level
    pub fn set_voltage(&mut self, voltage: VoltageLevel) {
        self.voltage = voltage;
    }

    /// Get pin mapping for current configuration
    pub fn get_pin_mapping(&self) -> Tsop48PinMapping {
        Tsop48PinMapping::for_pinout(self.pinout)
    }
}


/// TSOP-48 pin mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tsop48PinMapping {
    pub io0_7: [u8; 8],   // Data pins D0-D7
    pub io8_15: [u8; 8],  // Data pins D8-D15 (x16 mode)
    pub cle: u8,          // Command Latch Enable
    pub ale: u8,          // Address Latch Enable
    pub ce: u8,           // Chip Enable
    pub re: u8,           // Read Enable
    pub we: u8,           // Write Enable
    pub wp: u8,           // Write Protect
    pub rb: u8,           // Ready/Busy
}

impl Tsop48PinMapping {
    pub fn for_pinout(pinout: Tsop48Pinout) -> Self {
        match pinout {
            Tsop48Pinout::StandardNand => Self::standard(),
            Tsop48Pinout::Samsung => Self::samsung(),
            Tsop48Pinout::Hynix => Self::standard(), // Same as standard
            Tsop48Pinout::Micron => Self::standard(),
            Tsop48Pinout::Toshiba => Self::toshiba(),
        }
    }

    fn standard() -> Self {
        Self {
            io0_7: [29, 30, 31, 32, 41, 42, 43, 44],
            io8_15: [26, 27, 28, 33, 38, 39, 40, 45],
            cle: 17, ale: 18, ce: 9, re: 8, we: 19, wp: 20, rb: 7,
        }
    }

    fn samsung() -> Self {
        Self {
            io0_7: [29, 30, 31, 32, 41, 42, 43, 44],
            io8_15: [26, 27, 28, 33, 38, 39, 40, 45],
            cle: 17, ale: 18, ce: 9, re: 8, we: 19, wp: 20, rb: 7,
        }
    }

    fn toshiba() -> Self {
        Self {
            io0_7: [29, 30, 31, 32, 41, 42, 43, 44],
            io8_15: [26, 27, 28, 33, 38, 39, 40, 45],
            cle: 17, ale: 18, ce: 9, re: 8, we: 19, wp: 22, rb: 7,
        }
    }
}


// ============================================================================
// BGA Rework Station Integration
// ============================================================================

/// BGA rework station type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BgaStationType {
    /// Generic IR station
    GenericIr,
    /// Hot air station
    HotAir,
    /// JBC/Weller compatible
    Jbc,
    /// Quick 861DW and similar
    Quick,
    /// Hakko FR-810 and similar
    Hakko,
}

/// BGA rework station integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgaReworkStation {
    /// Station type
    pub station_type: BgaStationType,
    /// Communication port
    pub port: String,
    /// Current temperature (°C)
    pub current_temp: f32,
    /// Target temperature (°C)
    pub target_temp: f32,
    /// Heating active
    pub heating: bool,
    /// Profile name
    pub profile: Option<String>,
}

/// Temperature profile for BGA rework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgaProfile {
    /// Profile name
    pub name: String,
    /// Preheat temperature (°C)
    pub preheat_temp: f32,
    /// Preheat duration (seconds)
    pub preheat_duration: u32,
    /// Soak temperature (°C)
    pub soak_temp: f32,
    /// Soak duration (seconds)
    pub soak_duration: u32,
    /// Reflow temperature (°C)
    pub reflow_temp: f32,
    /// Reflow duration (seconds)
    pub reflow_duration: u32,
    /// Cooling rate (°C/s)
    pub cooling_rate: f32,
}

impl BgaProfile {
    /// Lead-free profile (SAC305)
    pub fn lead_free() -> Self {
        Self {
            name: "Lead-Free SAC305".into(),
            preheat_temp: 150.0,
            preheat_duration: 90,
            soak_temp: 200.0,
            soak_duration: 60,
            reflow_temp: 245.0,
            reflow_duration: 30,
            cooling_rate: 3.0,
        }
    }

    /// Leaded profile (Sn63/Pb37)
    pub fn leaded() -> Self {
        Self {
            name: "Leaded Sn63/Pb37".into(),
            preheat_temp: 140.0,
            preheat_duration: 60,
            soak_temp: 180.0,
            soak_duration: 45,
            reflow_temp: 215.0,
            reflow_duration: 20,
            cooling_rate: 2.5,
        }
    }
}


// ============================================================================
// Logic Analyzer Mode
// ============================================================================

/// Logic analyzer trigger type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    /// No trigger, immediate capture
    None,
    /// Rising edge on channel
    RisingEdge,
    /// Falling edge on channel
    FallingEdge,
    /// Any edge on channel
    AnyEdge,
    /// Pattern match
    Pattern,
    /// Protocol-specific (NAND command, SPI transaction)
    Protocol,
}

/// Logic analyzer channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicChannel {
    /// Channel number (0-15)
    pub number: u8,
    /// Channel name/label
    pub name: String,
    /// Enabled for capture
    pub enabled: bool,
    /// Trigger on this channel
    pub trigger: TriggerType,
}

/// Logic analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicAnalyzerConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Buffer size in samples
    pub buffer_size: u32,
    /// Pre-trigger samples (percentage)
    pub pre_trigger_percent: u8,
    /// Channels configuration
    pub channels: Vec<LogicChannel>,
    /// Trigger pattern (for Pattern trigger)
    pub trigger_pattern: Option<u16>,
    /// Trigger mask
    pub trigger_mask: u16,
}

impl Default for LogicAnalyzerConfig {
    fn default() -> Self {
        Self {
            sample_rate: 24_000_000, // 24 MHz
            buffer_size: 1_000_000,  // 1M samples
            pre_trigger_percent: 10,
            channels: (0..8).map(|i| LogicChannel {
                number: i,
                name: format!("D{}", i),
                enabled: true,
                trigger: TriggerType::None,
            }).collect(),
            trigger_pattern: None,
            trigger_mask: 0xFF,
        }
    }
}


/// Logic analyzer capture result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicCapture {
    /// Sample rate used
    pub sample_rate: u32,
    /// Number of samples captured
    pub sample_count: u32,
    /// Trigger position in samples
    pub trigger_position: u32,
    /// Captured data (packed bits)
    pub data: Vec<u8>,
    /// Channel count
    pub channel_count: u8,
}

impl LogicCapture {
    /// Get sample at index
    pub fn get_sample(&self, index: u32) -> Option<u16> {
        if index >= self.sample_count {
            return None;
        }
        let byte_idx = (index * self.channel_count as u32 / 8) as usize;
        if byte_idx >= self.data.len() {
            return None;
        }
        Some(self.data[byte_idx] as u16)
    }

    /// Export to VCD (Value Change Dump) format
    pub fn to_vcd(&self, channel_names: &[&str]) -> String {
        let mut vcd = String::new();
        vcd.push_str("$version OpenFlash Logic Analyzer $end\n");
        vcd.push_str(&format!("$timescale {}ns $end\n", 1_000_000_000 / self.sample_rate));
        vcd.push_str("$scope module capture $end\n");
        for (i, name) in channel_names.iter().enumerate() {
            vcd.push_str(&format!("$var wire 1 {} {} $end\n", (b'!' + i as u8) as char, name));
        }
        vcd.push_str("$upscope $end\n$enddefinitions $end\n");
        vcd
    }

    /// Export to Sigrok/PulseView format
    pub fn to_sigrok(&self) -> Vec<u8> {
        // Simplified - real implementation would create proper .sr file
        self.data.clone()
    }
}

/// Logic analyzer mode controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicAnalyzer {
    /// Configuration
    pub config: LogicAnalyzerConfig,
    /// Current state
    pub state: LogicAnalyzerState,
    /// Last capture
    pub last_capture: Option<LogicCapture>,
}

/// Logic analyzer state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicAnalyzerState {
    Idle,
    Armed,
    Capturing,
    Complete,
    Error,
}

impl Default for LogicAnalyzer {
    fn default() -> Self {
        Self {
            config: LogicAnalyzerConfig::default(),
            state: LogicAnalyzerState::Idle,
            last_capture: None,
        }
    }
}


impl LogicAnalyzer {
    /// Create with custom config
    pub fn with_config(config: LogicAnalyzerConfig) -> Self {
        Self {
            config,
            state: LogicAnalyzerState::Idle,
            last_capture: None,
        }
    }

    /// Arm the trigger
    pub fn arm(&mut self) -> HardwareResult<()> {
        self.state = LogicAnalyzerState::Armed;
        Ok(())
    }

    /// Start immediate capture
    pub fn capture(&mut self) -> HardwareResult<()> {
        self.state = LogicAnalyzerState::Capturing;
        Ok(())
    }

    /// Stop capture
    pub fn stop(&mut self) -> HardwareResult<()> {
        self.state = LogicAnalyzerState::Idle;
        Ok(())
    }

    /// NAND protocol decoder preset
    pub fn preset_nand(&mut self) {
        self.config.channels = vec![
            LogicChannel { number: 0, name: "CLE".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 1, name: "ALE".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 2, name: "WE#".into(), enabled: true, trigger: TriggerType::FallingEdge },
            LogicChannel { number: 3, name: "RE#".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 4, name: "CE#".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 5, name: "R/B#".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 6, name: "D0".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 7, name: "D7".into(), enabled: true, trigger: TriggerType::None },
        ];
    }

    /// SPI protocol decoder preset
    pub fn preset_spi(&mut self) {
        self.config.channels = vec![
            LogicChannel { number: 0, name: "CLK".into(), enabled: true, trigger: TriggerType::RisingEdge },
            LogicChannel { number: 1, name: "MOSI".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 2, name: "MISO".into(), enabled: true, trigger: TriggerType::None },
            LogicChannel { number: 3, name: "CS#".into(), enabled: true, trigger: TriggerType::FallingEdge },
        ];
    }
}


// ============================================================================
// JTAG/SWD Passthrough
// ============================================================================

/// JTAG TAP state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JtagState {
    TestLogicReset,
    RunTestIdle,
    SelectDrScan,
    CaptureDr,
    ShiftDr,
    Exit1Dr,
    PauseDr,
    Exit2Dr,
    UpdateDr,
    SelectIrScan,
    CaptureIr,
    ShiftIr,
    Exit1Ir,
    PauseIr,
    Exit2Ir,
    UpdateIr,
}

/// JTAG device in chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JtagDevice {
    /// Position in chain (0 = closest to TDI)
    pub position: u8,
    /// IDCODE
    pub idcode: u32,
    /// IR length
    pub ir_length: u8,
    /// Device name (if known)
    pub name: Option<String>,
    /// Manufacturer
    pub manufacturer: Option<String>,
}

impl JtagDevice {
    /// Parse IDCODE
    pub fn parse_idcode(idcode: u32) -> (u16, u16, u8) {
        let manufacturer = ((idcode >> 1) & 0x7FF) as u16;
        let part_number = ((idcode >> 12) & 0xFFFF) as u16;
        let version = ((idcode >> 28) & 0xF) as u8;
        (manufacturer, part_number, version)
    }

    /// Get manufacturer name from JEDEC ID
    pub fn manufacturer_name(jedec_id: u16) -> Option<&'static str> {
        match jedec_id {
            0x00E => Some("Intel"),
            0x01F => Some("Atmel"),
            0x020 => Some("STMicroelectronics"),
            0x049 => Some("Xilinx"),
            0x06E => Some("Altera"),
            0x093 => Some("Microchip"),
            0x0DD => Some("Samsung"),
            0x0C2 => Some("Macronix"),
            0x1C => Some("GigaDevice"),
            _ => None,
        }
    }
}


/// JTAG controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JtagController {
    /// Current TAP state
    pub state: JtagState,
    /// Clock frequency (Hz)
    pub frequency: u32,
    /// Devices in chain
    pub chain: Vec<JtagDevice>,
    /// Selected device index
    pub selected: Option<usize>,
}

impl Default for JtagController {
    fn default() -> Self {
        Self {
            state: JtagState::TestLogicReset,
            frequency: 1_000_000, // 1 MHz default
            chain: Vec::new(),
            selected: None,
        }
    }
}

impl JtagController {
    /// Scan JTAG chain and detect devices
    pub fn scan_chain(&mut self) -> HardwareResult<usize> {
        // In real implementation, this would scan the chain
        // For now, return empty chain
        self.chain.clear();
        Ok(0)
    }

    /// Reset TAP state machine
    pub fn reset(&mut self) {
        self.state = JtagState::TestLogicReset;
    }

    /// Write IR
    pub fn write_ir(&mut self, _ir: u32, _length: u8) -> HardwareResult<()> {
        self.state = JtagState::UpdateIr;
        Ok(())
    }

    /// Read/Write DR
    pub fn transfer_dr(&mut self, _data: &[u8], _length: u32) -> HardwareResult<Vec<u8>> {
        self.state = JtagState::UpdateDr;
        Ok(Vec::new())
    }

    /// Select device in chain
    pub fn select_device(&mut self, index: usize) -> HardwareResult<()> {
        if index >= self.chain.len() {
            return Err(HardwareError::JtagChainError("Device index out of range".into()));
        }
        self.selected = Some(index);
        Ok(())
    }
}


/// SWD (Serial Wire Debug) controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwdController {
    /// Clock frequency (Hz)
    pub frequency: u32,
    /// Target IDCODE
    pub idcode: Option<u32>,
    /// Connected
    pub connected: bool,
    /// Target in halt state
    pub halted: bool,
}

impl Default for SwdController {
    fn default() -> Self {
        Self {
            frequency: 1_000_000,
            idcode: None,
            connected: false,
            halted: false,
        }
    }
}

impl SwdController {
    /// Connect to target
    pub fn connect(&mut self) -> HardwareResult<u32> {
        // In real implementation, perform SWD init sequence
        self.connected = true;
        self.idcode = Some(0x2BA01477); // Example: Cortex-M4
        Ok(self.idcode.unwrap())
    }

    /// Disconnect from target
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.idcode = None;
    }

    /// Read AP/DP register
    pub fn read_register(&self, ap: bool, addr: u8) -> HardwareResult<u32> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        // Placeholder
        let _ = (ap, addr);
        Ok(0)
    }

    /// Write AP/DP register
    pub fn write_register(&mut self, ap: bool, addr: u8, value: u32) -> HardwareResult<()> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        let _ = (ap, addr, value);
        Ok(())
    }

    /// Read memory
    pub fn read_memory(&self, address: u32, size: usize) -> HardwareResult<Vec<u8>> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        let _ = address;
        Ok(vec![0; size])
    }

    /// Write memory
    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> HardwareResult<()> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        let _ = (address, data);
        Ok(())
    }

    /// Halt target
    pub fn halt(&mut self) -> HardwareResult<()> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        self.halted = true;
        Ok(())
    }

    /// Resume target
    pub fn resume(&mut self) -> HardwareResult<()> {
        if !self.connected {
            return Err(HardwareError::SwdError("Not connected".into()));
        }
        self.halted = false;
        Ok(())
    }
}


// ============================================================================
// OLED Display
// ============================================================================

/// OLED display type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OledType {
    /// SSD1306 128x64
    Ssd1306_128x64,
    /// SSD1306 128x32
    Ssd1306_128x32,
    /// SH1106 128x64
    Sh1106_128x64,
}

/// OLED display controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OledDisplay {
    /// Display type
    pub display_type: OledType,
    /// I2C address
    pub i2c_address: u8,
    /// Display enabled
    pub enabled: bool,
    /// Current brightness (0-255)
    pub brightness: u8,
    /// Inverted colors
    pub inverted: bool,
}

impl Default for OledDisplay {
    fn default() -> Self {
        Self {
            display_type: OledType::Ssd1306_128x64,
            i2c_address: 0x3C,
            enabled: true,
            brightness: 255,
            inverted: false,
        }
    }
}

impl OledDisplay {
    /// Get display dimensions
    pub fn dimensions(&self) -> (u8, u8) {
        match self.display_type {
            OledType::Ssd1306_128x64 | OledType::Sh1106_128x64 => (128, 64),
            OledType::Ssd1306_128x32 => (128, 32),
        }
    }

    /// Show status screen
    pub fn show_status(&self, status: &PcbStatus) -> Vec<u8> {
        let (w, h) = self.dimensions();
        let mut buffer = vec![0u8; (w as usize * h as usize) / 8];
        // Simplified - real implementation would render text
        let _ = (status, &mut buffer);
        buffer
    }

    /// Show progress bar
    pub fn show_progress(&self, label: &str, percent: u8) -> Vec<u8> {
        let (w, h) = self.dimensions();
        let mut buffer = vec![0u8; (w as usize * h as usize) / 8];
        let _ = (label, percent, &mut buffer);
        buffer
    }
}

/// PCB status for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbStatus {
    pub chip_detected: bool,
    pub chip_name: Option<String>,
    pub operation: Option<String>,
    pub progress: Option<u8>,
    pub wifi_connected: bool,
    pub usb_connected: bool,
}


// ============================================================================
// Protocol Commands for Hardware v2.1
// ============================================================================

/// Hardware-specific protocol commands (0xE0-0xEF)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareCommand {
    /// Detect PCB and get info
    PcbDetect = 0xE0,
    /// Get PCB capabilities
    PcbCapabilities = 0xE1,
    /// Set socket type
    SetSocket = 0xE2,
    /// Get adapter info
    AdapterInfo = 0xE3,
    /// Set adapter pinout
    SetPinout = 0xE4,
    /// Logic analyzer arm
    LogicArm = 0xE5,
    /// Logic analyzer capture
    LogicCapture = 0xE6,
    /// Logic analyzer get data
    LogicGetData = 0xE7,
    /// JTAG scan chain
    JtagScan = 0xE8,
    /// JTAG transfer
    JtagTransfer = 0xE9,
    /// SWD connect
    SwdConnect = 0xEA,
    /// SWD read/write
    SwdTransfer = 0xEB,
    /// OLED display update
    OledUpdate = 0xEC,
    /// Set voltage level
    SetVoltage = 0xED,
    /// BGA station control
    BgaControl = 0xEE,
    /// Get hardware status
    HardwareStatus = 0xEF,
}

impl HardwareCommand {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0xE0 => Some(Self::PcbDetect),
            0xE1 => Some(Self::PcbCapabilities),
            0xE2 => Some(Self::SetSocket),
            0xE3 => Some(Self::AdapterInfo),
            0xE4 => Some(Self::SetPinout),
            0xE5 => Some(Self::LogicArm),
            0xE6 => Some(Self::LogicCapture),
            0xE7 => Some(Self::LogicGetData),
            0xE8 => Some(Self::JtagScan),
            0xE9 => Some(Self::JtagTransfer),
            0xEA => Some(Self::SwdConnect),
            0xEB => Some(Self::SwdTransfer),
            0xEC => Some(Self::OledUpdate),
            0xED => Some(Self::SetVoltage),
            0xEE => Some(Self::BgaControl),
            0xEF => Some(Self::HardwareStatus),
            _ => None,
        }
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcb_revision_from_u8() {
        assert_eq!(PcbRevision::from_u8(0x01), Some(PcbRevision::RevA));
        assert_eq!(PcbRevision::from_u8(0x02), Some(PcbRevision::RevB));
        assert_eq!(PcbRevision::from_u8(0x03), Some(PcbRevision::RevC));
        assert_eq!(PcbRevision::from_u8(0xFF), None);
    }

    #[test]
    fn test_pcb_capabilities() {
        let pcb = OpenFlashPcb::default();
        let caps = pcb.capabilities();
        assert!(caps.parallel_nand);
        assert!(caps.spi_nand);
        assert!(caps.logic_analyzer);
    }

    #[test]
    fn test_tsop48_pin_mapping() {
        let mapping = Tsop48PinMapping::for_pinout(Tsop48Pinout::StandardNand);
        assert_eq!(mapping.cle, 17);
        assert_eq!(mapping.ale, 18);
        assert_eq!(mapping.ce, 9);
    }

    #[test]
    fn test_logic_analyzer_default() {
        let la = LogicAnalyzer::default();
        assert_eq!(la.state, LogicAnalyzerState::Idle);
        assert_eq!(la.config.sample_rate, 24_000_000);
        assert_eq!(la.config.channels.len(), 8);
    }

    #[test]
    fn test_logic_analyzer_presets() {
        let mut la = LogicAnalyzer::default();
        la.preset_nand();
        assert_eq!(la.config.channels[0].name, "CLE");
        assert_eq!(la.config.channels[2].name, "WE#");

        la.preset_spi();
        assert_eq!(la.config.channels[0].name, "CLK");
        assert_eq!(la.config.channels[3].name, "CS#");
    }

    #[test]
    fn test_jtag_idcode_parsing() {
        let idcode = 0x4BA00477; // Cortex-M4
        let (mfr, part, ver) = JtagDevice::parse_idcode(idcode);
        assert_eq!(mfr, 0x23B);
        assert_eq!(ver, 4);
        let _ = part;
    }

    #[test]
    fn test_swd_controller() {
        let mut swd = SwdController::default();
        assert!(!swd.connected);
        
        let result = swd.connect();
        assert!(result.is_ok());
        assert!(swd.connected);
        
        swd.disconnect();
        assert!(!swd.connected);
    }

    #[test]
    fn test_oled_dimensions() {
        let oled = OledDisplay::default();
        assert_eq!(oled.dimensions(), (128, 64));

        let oled32 = OledDisplay {
            display_type: OledType::Ssd1306_128x32,
            ..Default::default()
        };
        assert_eq!(oled32.dimensions(), (128, 32));
    }

    #[test]
    fn test_bga_profiles() {
        let lead_free = BgaProfile::lead_free();
        assert_eq!(lead_free.reflow_temp, 245.0);

        let leaded = BgaProfile::leaded();
        assert_eq!(leaded.reflow_temp, 215.0);
    }

    #[test]
    fn test_hardware_command_from_u8() {
        assert_eq!(HardwareCommand::from_u8(0xE0), Some(HardwareCommand::PcbDetect));
        assert_eq!(HardwareCommand::from_u8(0xE5), Some(HardwareCommand::LogicArm));
        assert_eq!(HardwareCommand::from_u8(0xE8), Some(HardwareCommand::JtagScan));
        assert_eq!(HardwareCommand::from_u8(0xEA), Some(HardwareCommand::SwdConnect));
        assert_eq!(HardwareCommand::from_u8(0xEF), Some(HardwareCommand::HardwareStatus));
        assert_eq!(HardwareCommand::from_u8(0xFF), None);
    }

    #[test]
    fn test_logic_capture_vcd_export() {
        let capture = LogicCapture {
            sample_rate: 24_000_000,
            sample_count: 100,
            trigger_position: 10,
            data: vec![0; 100],
            channel_count: 8,
        };
        let vcd = capture.to_vcd(&["CLK", "MOSI", "MISO", "CS"]);
        assert!(vcd.contains("OpenFlash Logic Analyzer"));
        assert!(vcd.contains("$timescale"));
    }

    #[test]
    fn test_voltage_levels() {
        let mut adapter = Tsop48Adapter::default();
        assert_eq!(adapter.voltage, VoltageLevel::V3_3);
        
        adapter.set_voltage(VoltageLevel::V1_8);
        assert_eq!(adapter.voltage, VoltageLevel::V1_8);
    }
}
