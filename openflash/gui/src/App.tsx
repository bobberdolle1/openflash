import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeFile, readFile } from "@tauri-apps/plugin-fs";
import { HexViewer } from "./components/HexViewer";
import { BitmapView } from "./components/BitmapView";
import "./styles.css";
import "./components/HexViewer.css";
import "./components/BitmapView.css";

interface DeviceInfo {
  id: string;
  name: string;
  serial: string | null;
  connected: boolean;
}

type FlashInterface = "ParallelNand" | "SpiNand";

interface ChipInfo {
  manufacturer: string;
  model: string;
  chip_id: number[];
  size_mb: number;
  page_size: number;
  block_size: number;
  interface: FlashInterface;
}

interface AnalysisResult {
  filesystem_type: string | null;
  signatures: { name: string; offset: number; confidence: number }[];
  bad_blocks: number[];
  empty_pages: number;
  data_pages: number;
}

type Tab = "operations" | "hexview" | "bitmap" | "analysis";

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

  const enableMock = useCallback(async () => {
    try {
      await invoke("enable_mock_mode");
      setMockEnabled(true);
      setStatus("Mock mode enabled");
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
      setStatus(`Switched to ${newInterface === "SpiNand" ? "SPI NAND" : "Parallel NAND"} mode`);
      
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

      const allData = new Uint8Array(totalPages * pageSize);
      let offset = 0;

      for (let page = 0; page < totalPages; page += chunkSize) {
        const numPages = Math.min(chunkSize, totalPages - page);
        const data = await invoke<number[]>("dump_nand", {
          startPage: page,
          numPages,
          pageSize,
        });

        allData.set(new Uint8Array(data), offset);
        offset += data.length;

        const progress = Math.round(((page + numPages) / totalPages) * 100);
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
              {!mockEnabled && (
                <button onClick={enableMock} className="secondary">
                  🧪 Mock
                </button>
              )}
            </div>
            <ul className="device-list">
              {devices.map((dev) => (
                <li key={dev.id} className={dev.connected ? "connected" : ""}>
                  <span>{dev.name}</span>
                  {dev.serial && <small>{dev.serial}</small>}
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
                  <small>Click Mock to test without hardware</small>
                </li>
              )}
            </ul>
          </section>

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
                >
                  Parallel
                </button>
                <button 
                  className={flashInterface === "SpiNand" ? "active" : ""}
                  onClick={() => switchInterface("SpiNand")}
                  disabled={isWorking}
                >
                  SPI
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
                    {chipInfo.interface === "SpiNand" ? "🔌 SPI" : "📊 Parallel"}
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
          </div>

          {/* Tab Content */}
          <div className="tab-content">
            {activeTab === "operations" && (
              <section className="panel operations">
                <h2>Flash Operations</h2>
                <div className="buttons">
                  <button onClick={startDump} disabled={!selectedDevice || isWorking}>
                    {isWorking ? "⏳ Dumping..." : "📥 Dump NAND"}
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
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
