import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeFile, readFile } from "@tauri-apps/plugin-fs";
import { HexViewer } from "./components/HexViewer";
import { BitmapView } from "./components/BitmapView";
import { AiAnalysis } from "./components/AiAnalysis";
import { SpiNorOperations } from "./components/SpiNorOperations";
import { UfsLunSelector } from "./components/UfsLunSelector";
import { PlatformInfo } from "./components/PlatformInfo";
import { NetworkDeviceDialog } from "./components/NetworkDeviceDialog";
import "./styles.css";
import "./components/HexViewer.css";
import "./components/BitmapView.css";
import "./components/AiAnalysis.css";
import "./components/SpiNorOperations.css";
import "./components/UfsLunSelector.css";
import "./components/PlatformInfo.css";
import "./components/NetworkDeviceDialog.css";

interface DeviceInfo {
  id: string;
  name: string;
  serial: string | null;
  connected: boolean;
  platform?: string;
  capabilities?: DeviceCapabilities;
  connection_type?: ConnectionType;
  protocol_version?: number;
  firmware_version?: string;
}

interface DeviceCapabilities {
  parallel_nand: boolean;
  spi_nand: boolean;
  spi_nor: boolean;
  emmc: boolean;
  nvddr: boolean;
  hardware_ecc: boolean;
  wifi: boolean;
  bluetooth: boolean;
  high_speed_usb: boolean;
}

type ConnectionType = 
  | "Usb"
  | { Tcp: { host: string; port: number } }
  | { UnixSocket: { path: string } };

type FlashInterface = "ParallelNand" | "SpiNand" | "SpiNor" | "Ufs" | "Emmc";

// UFS Logical Unit types
type UfsLunType = "UserData" | "BootA" | "BootB" | "Rpmb";

interface UfsLun {
  type: UfsLunType;
  capacity_bytes: number;
  block_size: number;
  enabled: boolean;
  write_protected: boolean;
}

interface ChipInfo {
  manufacturer: string;
  model: string;
  chip_id: number[];
  size_mb: number;
  page_size: number;
  block_size: number;
  interface: FlashInterface;
  // SPI NOR specific fields
  sector_size?: number;
  jedec_id?: number[];
  has_qspi?: boolean;
  has_dual?: boolean;
  voltage?: string;
  max_clock_mhz?: number;
  // Protection status for SPI NOR
  protected?: boolean;
  protection_bits?: {
    bp0: boolean;
    bp1: boolean;
    bp2: boolean;
    tb: boolean;
    sec: boolean;
    cmp: boolean;
  };
  // UFS specific fields
  luns?: UfsLun[];
  ufs_version?: string;
  serial_number?: string;
  boot_lun_enabled?: boolean;
}

interface AnalysisResult {
  filesystem_type: string | null;
  signatures: { name: string; offset: number; confidence: number }[];
  bad_blocks: number[];
  empty_pages: number;
  data_pages: number;
}

type Tab = "operations" | "hexview" | "bitmap" | "analysis" | "ai";

function App() {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [chipInfo, setChipInfo] = useState<ChipInfo | null>(null);
  const [status, setStatus] = useState<string>("Ready");
  const [dumpProgress, setDumpProgress] = useState<number>(0);
  const [dumpData, setDumpData] = useState<Uint8Array | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("operations");
  const [isWorking, setIsWorking] = useState(false);
  const [mockEnabled, setMockEnabled] = useState(false);
  const [hexHighlights, setHexHighlights] = useState<{ start: number; end: number; color: string; label?: string }[]>([]);
  const [flashInterface, setFlashInterface] = useState<FlashInterface>("ParallelNand");
  const [selectedUfsLun, setSelectedUfsLun] = useState<UfsLunType | null>(null);
  const [showNetworkDialog, setShowNetworkDialog] = useState(false);

  useEffect(() => {
    scanDevices();
  }, []);

  // Update highlights when analysis changes
  useEffect(() => {
    if (analysis?.signatures) {
      const highlights = analysis.signatures.map((sig, i) => ({
        start: sig.offset,
        end: sig.offset + 32, // Highlight 32 bytes
        color: `hsla(${(i * 60) % 360}, 70%, 50%, 0.3)`,
        label: sig.name,
      }));
      setHexHighlights(highlights);
    }
  }, [analysis]);

  // The emulator is an ordinary device in the list, so "demo mode" is just a
  // connection to it. It speaks the real protocol against an in-memory chip, so
  // the UI exercises the same code path as it does with hardware.
  const useEmulator = useCallback(async () => {
    try {
      await invoke("scan_devices");
      await invoke("connect_device", { deviceId: "emulated" });
      setMockEnabled(true);
      setStatus("Connected to the emulated chip (no hardware)");
      await scanDevices();
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }, []);

  async function switchInterface(newInterface: FlashInterface) {
    try {
      await invoke("set_interface", { interface: newInterface });
      setFlashInterface(newInterface);
      setChipInfo(null);
      
      const interfaceNames: Record<FlashInterface, string> = {
        ParallelNand: "Parallel NAND",
        SpiNand: "SPI NAND",
        SpiNor: "SPI NOR",
        Ufs: "UFS",
        Emmc: "eMMC",
      };
      setStatus(`Switched to ${interfaceNames[newInterface]} mode`);
      
      // Re-read chip info if connected
      if (selectedDevice) {
        await readChipInfo();
      }
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function scanDevices() {
    try {
      setStatus("Scanning...");
      const found = await invoke<DeviceInfo[]>("scan_devices");
      setDevices(found);
      setStatus(found.length > 0 ? `Found ${found.length} device(s)` : "No devices found");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function connectDevice(deviceId: string) {
    try {
      setStatus("Connecting...");
      await invoke("connect_device", { deviceId });
      setSelectedDevice(deviceId);
      
      const pong = await invoke<boolean>("ping");
      if (pong) {
        setStatus("Connected!");
        await readChipInfo();
      } else {
        setStatus("Connection failed");
      }
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function disconnectDevice() {
    try {
      await invoke("disconnect_device");
      setSelectedDevice(null);
      setChipInfo(null);
      setStatus("Disconnected");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function readChipInfo() {
    try {
      setStatus("Reading chip info...");
      const info = await invoke<ChipInfo>("get_chip_info");
      setChipInfo(info);
      setStatus("Chip info loaded");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function startDump() {
    if (!chipInfo || isWorking) return;

    try {
      setIsWorking(true);
      setStatus("Dumping...");
      setDumpProgress(0);
      setActiveTab("operations");

      const pageSize = chipInfo.page_size;
      const totalPages = Math.min(
        (chipInfo.size_mb * 1024 * 1024) / pageSize,
        65536
      );
      const chunkSize = 64;

      const total = totalPages * pageSize;
      const bytesPerRequest = chunkSize * pageSize;

      const allData = new Uint8Array(total);
      let offset = 0;

      while (offset < total) {
        const length = Math.min(bytesPerRequest, total - offset);
        const data = await invoke<number[]>("dump_range", {
          startAddress: offset,
          length,
        });

        allData.set(new Uint8Array(data), offset);
        offset += data.length;

        const progress = Math.round((offset / total) * 100);
        setDumpProgress(progress);
        setStatus(`Dumping... ${progress}%`);
      }

      setDumpData(allData);
      setDumpProgress(100);
      setStatus("Dump complete!");

      await analyzeData(allData);
    } catch (e) {
      setStatus(`Error: ${e}`);
    } finally {
      setIsWorking(false);
    }
  }

  async function analyzeData(data: Uint8Array) {
    if (!chipInfo) return;

    try {
      setStatus("Analyzing...");
      const result = await invoke<AnalysisResult>("analyze_dump", {
        data: Array.from(data),
        pageSize: chipInfo.page_size,
        pagesPerBlock: chipInfo.block_size,
      });
      setAnalysis(result);
      setStatus("Analysis complete");
      setActiveTab("analysis");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function saveDump() {
    if (!dumpData) return;

    try {
      const path = await save({
        filters: [
          { name: "Binary", extensions: ["bin"] },
          { name: "All Files", extensions: ["*"] },
        ],
        defaultPath: `dump_${chipInfo?.model || "nand"}.bin`,
      });

      if (path) {
        await writeFile(path, dumpData);
        setStatus(`Saved to ${path}`);
        await invoke("add_recent_file", { path });
      }
    } catch (e) {
      setStatus(`Error saving: ${e}`);
    }
  }

  async function loadDump() {
    try {
      const path = await open({
        filters: [
          { name: "Binary", extensions: ["bin"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });

      if (path) {
        setStatus("Loading...");
        const data = await readFile(path);
        setDumpData(new Uint8Array(data));
        setStatus(`Loaded ${formatBytes(data.length)}`);
        await invoke("add_recent_file", { path });
        
        // Auto-analyze
        const result = await invoke<AnalysisResult>("analyze_dump", {
          data: Array.from(data),
          pageSize: chipInfo?.page_size || 2048,
          pagesPerBlock: chipInfo?.block_size || 64,
        });
        setAnalysis(result);
        setActiveTab("analysis");
      }
    } catch (e) {
      setStatus(`Error loading: ${e}`);
    }
  }

  const handleBitmapPageSelect = useCallback((pageIndex: number) => {
    if (chipInfo) {
      const offset = pageIndex * chipInfo.page_size;
      // Could navigate to hex view at this offset
      console.log(`Selected page ${pageIndex} at offset 0x${offset.toString(16)}`);
    }
  }, [chipInfo]);

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  // SPI NOR operation handler
  async function handleSpiNorOperation(operation: string, address?: number) {
    try {
      setIsWorking(true);
      switch (operation) {
        case "sector_erase":
          await invoke("spi_nor_sector_erase", { address });
          break;
        case "block_erase":
          await invoke("spi_nor_block_erase", { address });
          break;
        case "chip_erase":
          await invoke("spi_nor_chip_erase");
          break;
        case "unlock_all":
          await invoke("spi_nor_unlock_all");
          // Refresh chip info to update protection status
          await readChipInfo();
          break;
        default:
          throw new Error(`Unknown operation: ${operation}`);
      }
    } finally {
      setIsWorking(false);
    }
  }

  return (
    <div className="app">
      <header className="header">
        <h1>OpenFlash</h1>
        <span className="status">{status}</span>
      </header>

      <main className="main">
        {/* Sidebar */}
        <aside className="sidebar">
          <section className="panel devices">
            <h2>Devices</h2>
            <div className="buttons">
              <button onClick={scanDevices} disabled={isWorking}>
                🔄 Scan
              </button>
              <button onClick={() => setShowNetworkDialog(true)} className="secondary" title="Add network device (SBC)">
                🌐 Network
              </button>
              {!mockEnabled && (
                <button onClick={useEmulator} className="secondary">
                  🧪 Mock
                </button>
              )}
            </div>
            <ul className="device-list">
              {devices.map((dev) => (
                <li key={dev.id} className={dev.connected ? "connected" : ""}>
                  <div className="device-info">
                    <span className="device-name">
                      {dev.name}
                      {dev.connection_type && typeof dev.connection_type === "object" && "Tcp" in dev.connection_type && (
                        <span className="network-badge" title="Network device">🌐</span>
                      )}
                    </span>
                    {dev.serial && <small className="device-serial">{dev.serial}</small>}
                    {dev.protocol_version && (
                      <small className="device-protocol">v{dev.protocol_version.toString(16).toUpperCase()}</small>
                    )}
                  </div>
                  {dev.connected ? (
                    <button onClick={disconnectDevice} disabled={isWorking} className="secondary">
                      Disconnect
                    </button>
                  ) : (
                    <button onClick={() => connectDevice(dev.id)} disabled={isWorking}>
                      Connect
                    </button>
                  )}
                </li>
              ))}
              {devices.length === 0 && (
                <li className="empty">
                  No devices found<br />
                  <small>Click Mock to test, or Network for SBC</small>
                </li>
              )}
            </ul>
          </section>

          {/* Platform Info - shown when connected */}
          {selectedDevice && (
            <PlatformInfo 
              connected={!!selectedDevice} 
              onStatusChange={setStatus}
            />
          )}

          <section className="panel chip-info">
            <h2>Chip Info</h2>
            
            {/* Interface selector */}
            <div className="interface-selector">
              <label>Interface:</label>
              <div className="toggle-buttons">
                <button 
                  className={flashInterface === "ParallelNand" ? "active" : ""}
                  onClick={() => switchInterface("ParallelNand")}
                  disabled={isWorking}
                  title="Parallel NAND Flash"
                >
                  Parallel
                </button>
                <button 
                  className={flashInterface === "SpiNand" ? "active" : ""}
                  onClick={() => switchInterface("SpiNand")}
                  disabled={isWorking}
                  title="SPI NAND Flash"
                >
                  SPI NAND
                </button>
                <button 
                  className={flashInterface === "SpiNor" ? "active" : ""}
                  onClick={() => switchInterface("SpiNor")}
                  disabled={isWorking}
                  title="SPI NOR Flash"
                >
                  SPI NOR
                </button>
                <button 
                  className={flashInterface === "Emmc" ? "active" : ""}
                  onClick={() => switchInterface("Emmc")}
                  disabled={isWorking}
                  title="eMMC Storage"
                >
                  eMMC
                </button>
                <button 
                  className={flashInterface === "Ufs" ? "active" : ""}
                  onClick={() => switchInterface("Ufs")}
                  disabled={isWorking}
                  title="Universal Flash Storage"
                >
                  UFS
                </button>
              </div>
            </div>
            
            {chipInfo ? (
              <div className="info-grid compact">
                <div>
                  <label>Manufacturer</label>
                  <span>{chipInfo.manufacturer}</span>
                </div>
                <div>
                  <label>Model</label>
                  <span>{chipInfo.model}</span>
                </div>
                <div>
                  <label>Size</label>
                  <span>{chipInfo.size_mb} MB</span>
                </div>
                <div>
                  <label>Page</label>
                  <span>{chipInfo.page_size} B</span>
                </div>
                <div>
                  <label>Block</label>
                  <span>{chipInfo.block_size} pages</span>
                </div>
                <div>
                  <label>Interface</label>
                  <span className="interface-badge">
                    {chipInfo.interface === "SpiNand" && "🔌 SPI NAND"}
                    {chipInfo.interface === "ParallelNand" && "📊 Parallel"}
                    {chipInfo.interface === "SpiNor" && "💾 SPI NOR"}
                    {chipInfo.interface === "Emmc" && "📦 eMMC"}
                    {chipInfo.interface === "Ufs" && "⚡ UFS"}
                  </span>
                </div>
                <div>
                  <label>Chip ID</label>
                  <span className="mono">
                    {chipInfo.chip_id.map((b) => b.toString(16).padStart(2, "0")).join(" ")}
                  </span>
                </div>
              </div>
            ) : (
              <p className="empty">Connect a device to see chip info</p>
            )}
          </section>
        </aside>

        {/* Main Content */}
        <div className="content">
          {/* Tabs */}
          <div className="tabs">
            <button 
              className={activeTab === "operations" ? "active" : ""} 
              onClick={() => setActiveTab("operations")}
            >
              ⚡ Operations
            </button>
            <button 
              className={activeTab === "hexview" ? "active" : ""} 
              onClick={() => setActiveTab("hexview")}
              disabled={!dumpData}
            >
              📝 Hex View
            </button>
            <button 
              className={activeTab === "bitmap" ? "active" : ""} 
              onClick={() => setActiveTab("bitmap")}
              disabled={!dumpData}
            >
              🗺️ Bitmap
            </button>
            <button 
              className={activeTab === "analysis" ? "active" : ""} 
              onClick={() => setActiveTab("analysis")}
              disabled={!analysis}
            >
              🔬 Analysis
            </button>
            <button 
              className={activeTab === "ai" ? "active" : ""} 
              onClick={() => setActiveTab("ai")}
              disabled={!dumpData}
            >
              🤖 AI
            </button>
          </div>

          {/* Tab Content */}
          <div className="tab-content">
            {activeTab === "operations" && (
              <section className="panel operations">
                <h2>Flash Operations</h2>
                <div className="buttons">
                  <button onClick={startDump} disabled={!selectedDevice || isWorking}>
                    {isWorking ? "⏳ Dumping..." : "📥 Dump Flash"}
                  </button>
                  <button onClick={saveDump} disabled={!dumpData || isWorking} className="secondary">
                    💾 Save Dump
                  </button>
                  <button onClick={loadDump} disabled={isWorking} className="secondary">
                    📂 Load Dump
                  </button>
                </div>
                
                {dumpProgress > 0 && (
                  <div className="progress">
                    <div className="progress-bar" style={{ width: `${dumpProgress}%` }} />
                    <span>{dumpProgress}%</span>
                  </div>
                )}

                {dumpData && (
                  <div className="dump-info">
                    <p>Dump size: {formatBytes(dumpData.length)}</p>
                  </div>
                )}

                {/* SPI NOR specific operations */}
                {chipInfo && chipInfo.interface === "SpiNor" && (
                  <SpiNorOperations
                    chipInfo={chipInfo}
                    onOperation={handleSpiNorOperation}
                    disabled={!selectedDevice || isWorking}
                    onStatusChange={setStatus}
                  />
                )}

                {/* UFS LUN selector */}
                {chipInfo && chipInfo.interface === "Ufs" && chipInfo.luns && (
                  <UfsLunSelector
                    luns={chipInfo.luns}
                    selectedLun={selectedUfsLun}
                    onLunSelect={setSelectedUfsLun}
                    ufsVersion={chipInfo.ufs_version}
                    disabled={!selectedDevice || isWorking}
                    onStatusChange={setStatus}
                  />
                )}

                {!selectedDevice && !dumpData && (
                  <div className="empty" style={{ marginTop: "2rem" }}>
                    <p>Connect a device or load a dump file to get started</p>
                  </div>
                )}
              </section>
            )}

            {activeTab === "hexview" && dumpData && (
              <section className="panel hex-panel">
                <HexViewer 
                  data={dumpData} 
                  bytesPerRow={16} 
                  pageSize={512}
                  highlights={hexHighlights}
                />
              </section>
            )}

            {activeTab === "bitmap" && dumpData && (
              <section className="panel bitmap-panel">
                <BitmapView 
                  data={dumpData} 
                  width={256} 
                  pageSize={chipInfo?.page_size || 2048}
                  onPageSelect={handleBitmapPageSelect}
                />
              </section>
            )}

            {activeTab === "analysis" && analysis && (
              <section className="panel analysis">
                <h2>Analysis Results</h2>
                <div className="analysis-results">
                  <div>
                    <label>Filesystem</label>
                    <span className={analysis.filesystem_type ? "detected" : ""}>
                      {analysis.filesystem_type || "Unknown"}
                    </span>
                  </div>
                  <div>
                    <label>Data Pages</label>
                    <span>{analysis.data_pages.toLocaleString()}</span>
                  </div>
                  <div>
                    <label>Empty Pages</label>
                    <span>{analysis.empty_pages.toLocaleString()}</span>
                  </div>
                  <div>
                    <label>Bad Blocks</label>
                    <span className={analysis.bad_blocks.length > 0 ? "warning" : ""}>
                      {analysis.bad_blocks.length}
                    </span>
                  </div>
                  
                  {analysis.signatures.length > 0 && (
                    <div className="signatures">
                      <label>Detected Signatures</label>
                      <ul>
                        {analysis.signatures.map((sig, i) => (
                          <li key={i}>
                            <span className="sig-name">{sig.name}</span>
                            <span className="sig-offset">0x{sig.offset.toString(16).toUpperCase()}</span>
                            <span className="sig-conf">{Math.round(sig.confidence * 100)}%</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {analysis.bad_blocks.length > 0 && (
                    <div className="bad-blocks">
                      <label>Bad Block Map</label>
                      <div className="bad-block-list">
                        {analysis.bad_blocks.slice(0, 20).map((block, i) => (
                          <span key={i} className="bad-block">#{block}</span>
                        ))}
                        {analysis.bad_blocks.length > 20 && (
                          <span className="more">+{analysis.bad_blocks.length - 20} more</span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </section>
            )}

            {activeTab === "ai" && (
              <section className="panel ai-panel">
                <AiAnalysis 
                  data={dumpData}
                  pageSize={chipInfo?.page_size || 2048}
                  pagesPerBlock={chipInfo?.block_size || 64}
                  onPatternSelect={(offset) => {
                    console.log(`Navigate to offset 0x${offset.toString(16)}`);
                    setActiveTab("hexview");
                  }}
                />
              </section>
            )}
          </div>
        </div>
      </main>

      {/* Network Device Dialog */}
      <NetworkDeviceDialog
        isOpen={showNetworkDialog}
        onClose={() => setShowNetworkDialog(false)}
        onDeviceAdded={scanDevices}
        onStatusChange={setStatus}
      />
    </div>
  );
}

export default App;
