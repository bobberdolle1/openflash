import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import "./AiAnalysis.css";

interface PatternInfo {
  pattern_type: string;
  start_offset: number;
  end_offset: number;
  confidence: string;
  description: string;
}

interface AnomalyInfo {
  severity: string;
  location: number | null;
  description: string;
  recommendation: string;
}

interface RecoverySuggestionInfo {
  priority: number;
  action: string;
  description: string;
  estimated_success: number;
}

interface ChipRecommendationInfo {
  category: string;
  title: string;
  description: string;
  importance: number;
}

// v1.4 new interfaces
interface FilesystemInfo {
  fs_type: string;
  offset: number;
  size: number | null;
  confidence: string;
}

interface OobAnalysisInfo {
  oob_size: number;
  ecc_scheme: string;
  ecc_offset: number;
  ecc_size: number;
  bad_block_marker_offset: number;
  confidence: string;
}

interface KeyCandidateInfo {
  offset: number;
  key_type: string;
  key_length: number;
  entropy: number;
  confidence: string;
  context: string;
}

interface WearAnalysisInfo {
  hottest_blocks: number[];
  coldest_blocks: number[];
  min_erases: number;
  max_erases: number;
  avg_erases: number;
  estimated_remaining_life_percent: number;
  recommendations: string[];
}

interface MemoryMapRegionInfo {
  start: number;
  end: number;
  region_type: string;
  name: string;
  color: string;
}

interface MemoryMapInfo {
  total_size: number;
  regions: MemoryMapRegionInfo[];
}

interface AiAnalysisResponse {
  patterns: PatternInfo[];
  anomalies: AnomalyInfo[];
  recovery_suggestions: RecoverySuggestionInfo[];
  chip_recommendations: ChipRecommendationInfo[];
  data_quality_score: number;
  encryption_probability: number;
  compression_probability: number;
  summary: string;
  // v1.4 additions
  filesystems: FilesystemInfo[];
  oob_analysis: OobAnalysisInfo | null;
  key_candidates: KeyCandidateInfo[];
  wear_analysis: WearAnalysisInfo | null;
  memory_map: MemoryMapInfo | null;
}

interface AiAnalysisProps {
  data: Uint8Array | null;
  pageSize: number;
  pagesPerBlock: number;
  onPatternSelect?: (offset: number) => void;
}

type AiSection = "overview" | "anomalies" | "recovery" | "recommendations" | "filesystems" | "oob" | "keys" | "wear" | "map";

export function AiAnalysis({ data, pageSize, pagesPerBlock, onPatternSelect }: AiAnalysisProps) {
  const [analysis, setAnalysis] = useState<AiAnalysisResponse | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<AiSection>("overview");
  const [isExporting, setIsExporting] = useState(false);

  useEffect(() => {
    if (data && data.length > 0) {
      runAnalysis();
    }
  }, [data]);

  async function runAnalysis() {
    if (!data) return;
    
    setIsAnalyzing(true);
    setError(null);
    
    try {
      const result = await invoke<AiAnalysisResponse>("ai_analyze_dump", {
        data: Array.from(data),
        pageSize,
        pagesPerBlock,
      });
      setAnalysis(result);
    } catch (e) {
      setError(`Analysis failed: ${e}`);
    } finally {
      setIsAnalyzing(false);
    }
  }

  async function exportReport() {
    if (!data) return;
    
    setIsExporting(true);
    try {
      const report = await invoke<string>("ai_generate_report", {
        data: Array.from(data),
        pageSize,
        pagesPerBlock,
      });
      
      const path = await save({
        filters: [{ name: "Markdown", extensions: ["md"] }],
        defaultPath: "ai_analysis_report.md",
      });
      
      if (path) {
        await writeTextFile(path, report);
      }
    } catch (e) {
      setError(`Export failed: ${e}`);
    } finally {
      setIsExporting(false);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function getPatternIcon(type: string): string {
    const icons: Record<string, string> = {
      "Encrypted": "🔐",
      "Compressed": "📦",
      "Executable": "⚙️",
      "Text": "📝",
      "Empty": "⬜",
      "Zeroed": "0️⃣",
      "Repeating": "🔄",
      "StructuredBinary": "📊",
      "FilesystemMeta": "📁",
      "Random": "🎲",
      "BootLoader": "🚀",
      "Kernel": "🐧",
      "DeviceTree": "🌳",
      "ConfigData": "⚙️",
      "OobData": "📋",
      "WearLevelMeta": "📈",
    };
    return icons[type] || "❓";
  }

  function getSeverityColor(severity: string): string {
    switch (severity) {
      case "Critical": return "#ff4444";
      case "Warning": return "#ffaa00";
      case "Info": return "#4488ff";
      default: return "#888";
    }
  }

  function getConfidenceColor(confidence: string): string {
    switch (confidence) {
      case "VeryHigh": return "#00cc66";
      case "High": return "#88cc00";
      case "Medium": return "#ffaa00";
      case "Low": return "#ff6644";
      default: return "#888";
    }
  }

  if (!data) {
    return (
      <div className="ai-analysis empty">
        <div className="ai-empty-state">
          <span className="ai-icon">🤖</span>
          <h3>AI Analysis v1.4</h3>
          <p>Load or dump data to enable AI-powered analysis</p>
          <ul className="ai-features">
            <li>🔍 Pattern Recognition</li>
            <li>📁 Filesystem Detection</li>
            <li>🔐 Key Search</li>
            <li>📊 Wear Analysis</li>
            <li>🗺️ Memory Map</li>
          </ul>
        </div>
      </div>
    );
  }

  if (isAnalyzing) {
    return (
      <div className="ai-analysis loading">
        <div className="ai-loading-state">
          <div className="ai-spinner"></div>
          <h3>Analyzing...</h3>
          <p>AI is examining {formatBytes(data.length)} of data</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="ai-analysis error">
        <div className="ai-error-state">
          <span className="ai-icon">⚠️</span>
          <h3>Analysis Error</h3>
          <p>{error}</p>
          <button onClick={runAnalysis}>Retry</button>
        </div>
      </div>
    );
  }

  if (!analysis) {
    return null;
  }

  return (
    <div className="ai-analysis">
      {/* Summary Card */}
      <div className="ai-summary-card">
        <div className="ai-summary-header">
          <span className="ai-icon">🤖</span>
          <h3>AI Analysis v1.4</h3>
          <div className="ai-header-actions">
            <button 
              className="ai-export" 
              onClick={exportReport} 
              disabled={isExporting}
              title="Export Report"
            >
              {isExporting ? "⏳" : "📄"}
            </button>
            <button className="ai-refresh" onClick={runAnalysis} title="Re-analyze">
              🔄
            </button>
          </div>
        </div>
        <p className="ai-summary-text">{analysis.summary}</p>
        
        <div className="ai-metrics">
          <div className="ai-metric">
            <label>Data Quality</label>
            <div className="ai-progress-bar">
              <div 
                className="ai-progress-fill quality"
                style={{ width: `${analysis.data_quality_score * 100}%` }}
              />
            </div>
            <span>{Math.round(analysis.data_quality_score * 100)}%</span>
          </div>
          
          <div className="ai-metric">
            <label>Encryption</label>
            <div className="ai-progress-bar">
              <div 
                className="ai-progress-fill encryption"
                style={{ width: `${analysis.encryption_probability * 100}%` }}
              />
            </div>
            <span>{Math.round(analysis.encryption_probability * 100)}%</span>
          </div>
          
          <div className="ai-metric">
            <label>Compression</label>
            <div className="ai-progress-bar">
              <div 
                className="ai-progress-fill compression"
                style={{ width: `${analysis.compression_probability * 100}%` }}
              />
            </div>
            <span>{Math.round(analysis.compression_probability * 100)}%</span>
          </div>

          {analysis.wear_analysis && (
            <div className="ai-metric">
              <label>Flash Life</label>
              <div className="ai-progress-bar">
                <div 
                  className="ai-progress-fill life"
                  style={{ width: `${analysis.wear_analysis.estimated_remaining_life_percent}%` }}
                />
              </div>
              <span>{Math.round(analysis.wear_analysis.estimated_remaining_life_percent)}%</span>
            </div>
          )}
        </div>
      </div>

      {/* Section Tabs */}
      <div className="ai-tabs">
        <button 
          className={activeSection === "overview" ? "active" : ""}
          onClick={() => setActiveSection("overview")}
        >
          📊 Patterns ({analysis.patterns.length})
        </button>
        <button 
          className={activeSection === "anomalies" ? "active" : ""}
          onClick={() => setActiveSection("anomalies")}
        >
          ⚠️ Issues ({analysis.anomalies.length})
        </button>
        <button 
          className={activeSection === "filesystems" ? "active" : ""}
          onClick={() => setActiveSection("filesystems")}
        >
          📁 FS ({analysis.filesystems.length})
        </button>
        <button 
          className={activeSection === "oob" ? "active" : ""}
          onClick={() => setActiveSection("oob")}
          disabled={!analysis.oob_analysis}
        >
          📋 OOB
        </button>
        <button 
          className={activeSection === "keys" ? "active" : ""}
          onClick={() => setActiveSection("keys")}
        >
          🔐 Keys ({analysis.key_candidates.length})
        </button>
        <button 
          className={activeSection === "wear" ? "active" : ""}
          onClick={() => setActiveSection("wear")}
          disabled={!analysis.wear_analysis}
        >
          📈 Wear
        </button>
        <button 
          className={activeSection === "map" ? "active" : ""}
          onClick={() => setActiveSection("map")}
          disabled={!analysis.memory_map}
        >
          🗺️ Map
        </button>
        <button 
          className={activeSection === "recovery" ? "active" : ""}
          onClick={() => setActiveSection("recovery")}
        >
          🔧 Recovery ({analysis.recovery_suggestions.length})
        </button>
        <button 
          className={activeSection === "recommendations" ? "active" : ""}
          onClick={() => setActiveSection("recommendations")}
        >
          💡 Tips ({analysis.chip_recommendations.length})
        </button>
      </div>

      {/* Section Content */}
      <div className="ai-section-content">
        {activeSection === "overview" && (
          <div className="ai-patterns">
            {analysis.patterns.length === 0 ? (
              <p className="ai-empty">No patterns detected</p>
            ) : (
              <div className="ai-pattern-list">
                {analysis.patterns.map((pattern, i) => (
                  <div 
                    key={i} 
                    className="ai-pattern-item"
                    onClick={() => onPatternSelect?.(pattern.start_offset)}
                  >
                    <span className="pattern-icon">
                      {getPatternIcon(pattern.pattern_type)}
                    </span>
                    <div className="pattern-info">
                      <div className="pattern-header">
                        <span className="pattern-type">{pattern.pattern_type}</span>
                        <span 
                          className="pattern-confidence"
                          style={{ color: getConfidenceColor(pattern.confidence) }}
                        >
                          {pattern.confidence}
                        </span>
                      </div>
                      <p className="pattern-desc">{pattern.description}</p>
                      <div className="pattern-range">
                        <span>0x{pattern.start_offset.toString(16).toUpperCase()}</span>
                        <span>→</span>
                        <span>0x{pattern.end_offset.toString(16).toUpperCase()}</span>
                        <span className="pattern-size">
                          ({formatBytes(pattern.end_offset - pattern.start_offset)})
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeSection === "anomalies" && (
          <div className="ai-anomalies">
            {analysis.anomalies.length === 0 ? (
              <p className="ai-empty">✅ No issues detected</p>
            ) : (
              <div className="ai-anomaly-list">
                {analysis.anomalies.map((anomaly, i) => (
                  <div 
                    key={i} 
                    className="ai-anomaly-item"
                    style={{ borderLeftColor: getSeverityColor(anomaly.severity) }}
                  >
                    <div className="anomaly-header">
                      <span 
                        className="anomaly-severity"
                        style={{ color: getSeverityColor(anomaly.severity) }}
                      >
                        {anomaly.severity}
                      </span>
                      {anomaly.location !== null && (
                        <span className="anomaly-location">
                          @ 0x{anomaly.location.toString(16).toUpperCase()}
                        </span>
                      )}
                    </div>
                    <p className="anomaly-desc">{anomaly.description}</p>
                    <p className="anomaly-recommendation">
                      💡 {anomaly.recommendation}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeSection === "filesystems" && (
          <div className="ai-filesystems">
            {analysis.filesystems.length === 0 ? (
              <p className="ai-empty">No filesystems detected</p>
            ) : (
              <div className="ai-fs-list">
                {analysis.filesystems.map((fs, i) => (
                  <div 
                    key={i} 
                    className="ai-fs-item"
                    onClick={() => onPatternSelect?.(fs.offset)}
                  >
                    <span className="fs-icon">📁</span>
                    <div className="fs-info">
                      <div className="fs-header">
                        <span className="fs-type">{fs.fs_type}</span>
                        <span 
                          className="fs-confidence"
                          style={{ color: getConfidenceColor(fs.confidence) }}
                        >
                          {fs.confidence}
                        </span>
                      </div>
                      <div className="fs-details">
                        <span>Offset: 0x{fs.offset.toString(16).toUpperCase()}</span>
                        {fs.size && <span>Size: {formatBytes(fs.size)}</span>}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeSection === "oob" && analysis.oob_analysis && (
          <div className="ai-oob">
            <div className="oob-card">
              <h4>OOB/Spare Area Analysis</h4>
              <div className="oob-grid">
                <div className="oob-item">
                  <label>OOB Size</label>
                  <span>{analysis.oob_analysis.oob_size} bytes</span>
                </div>
                <div className="oob-item">
                  <label>ECC Scheme</label>
                  <span className="ecc-badge">{analysis.oob_analysis.ecc_scheme}</span>
                </div>
                <div className="oob-item">
                  <label>ECC Offset</label>
                  <span>{analysis.oob_analysis.ecc_offset}</span>
                </div>
                <div className="oob-item">
                  <label>ECC Size</label>
                  <span>{analysis.oob_analysis.ecc_size} bytes</span>
                </div>
                <div className="oob-item">
                  <label>Bad Block Marker</label>
                  <span>Offset {analysis.oob_analysis.bad_block_marker_offset}</span>
                </div>
                <div className="oob-item">
                  <label>Confidence</label>
                  <span style={{ color: getConfidenceColor(analysis.oob_analysis.confidence) }}>
                    {analysis.oob_analysis.confidence}
                  </span>
                </div>
              </div>
              
              <div className="oob-layout">
                <h5>OOB Layout</h5>
                <div className="oob-visual">
                  <div 
                    className="oob-region bbm" 
                    style={{ width: '10%' }}
                    title="Bad Block Marker"
                  >
                    BBM
                  </div>
                  <div 
                    className="oob-region ecc" 
                    style={{ width: `${(analysis.oob_analysis.ecc_size / analysis.oob_analysis.oob_size) * 100}%` }}
                    title="ECC Data"
                  >
                    ECC
                  </div>
                  <div 
                    className="oob-region user" 
                    style={{ flex: 1 }}
                    title="User Data"
                  >
                    User
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeSection === "keys" && (
          <div className="ai-keys">
            {analysis.key_candidates.length === 0 ? (
              <div className="ai-empty">
                <p>No encryption keys detected</p>
                <small>Enable deep scan for thorough key search</small>
              </div>
            ) : (
              <div className="ai-key-list">
                {analysis.key_candidates.map((key, i) => (
                  <div 
                    key={i} 
                    className="ai-key-item"
                    onClick={() => onPatternSelect?.(key.offset)}
                  >
                    <span className="key-icon">🔐</span>
                    <div className="key-info">
                      <div className="key-header">
                        <span className="key-type">{key.key_type}</span>
                        <span 
                          className="key-confidence"
                          style={{ color: getConfidenceColor(key.confidence) }}
                        >
                          {key.confidence}
                        </span>
                      </div>
                      <div className="key-details">
                        <span>Offset: 0x{key.offset.toString(16).toUpperCase()}</span>
                        <span>Length: {key.key_length} bytes</span>
                        <span>Entropy: {key.entropy.toFixed(2)}</span>
                      </div>
                      <p className="key-context">{key.context}</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeSection === "wear" && analysis.wear_analysis && (
          <div className="ai-wear">
            <div className="wear-card">
              <h4>Wear Leveling Analysis</h4>
              
              <div className="wear-life">
                <div className="life-circle">
                  <svg viewBox="0 0 100 100">
                    <circle 
                      cx="50" cy="50" r="45" 
                      fill="none" 
                      stroke="#333" 
                      strokeWidth="8"
                    />
                    <circle 
                      cx="50" cy="50" r="45" 
                      fill="none" 
                      stroke={analysis.wear_analysis.estimated_remaining_life_percent > 50 ? "#00cc66" : "#ff6644"}
                      strokeWidth="8"
                      strokeDasharray={`${analysis.wear_analysis.estimated_remaining_life_percent * 2.83} 283`}
                      transform="rotate(-90 50 50)"
                    />
                  </svg>
                  <div className="life-text">
                    <span className="life-percent">
                      {Math.round(analysis.wear_analysis.estimated_remaining_life_percent)}%
                    </span>
                    <span className="life-label">Life</span>
                  </div>
                </div>
              </div>
              
              <div className="wear-stats">
                <div className="wear-stat">
                  <label>Min Erases</label>
                  <span>{analysis.wear_analysis.min_erases}</span>
                </div>
                <div className="wear-stat">
                  <label>Max Erases</label>
                  <span>{analysis.wear_analysis.max_erases}</span>
                </div>
                <div className="wear-stat">
                  <label>Avg Erases</label>
                  <span>{analysis.wear_analysis.avg_erases.toFixed(1)}</span>
                </div>
              </div>
              
              <div className="wear-blocks">
                <div className="wear-block-section">
                  <h5>🔥 Hottest Blocks</h5>
                  <div className="block-list hot">
                    {analysis.wear_analysis.hottest_blocks.slice(0, 5).map((block, i) => (
                      <span key={i} className="block-badge">#{block}</span>
                    ))}
                  </div>
                </div>
                <div className="wear-block-section">
                  <h5>❄️ Coldest Blocks</h5>
                  <div className="block-list cold">
                    {analysis.wear_analysis.coldest_blocks.slice(0, 5).map((block, i) => (
                      <span key={i} className="block-badge">#{block}</span>
                    ))}
                  </div>
                </div>
              </div>
              
              {analysis.wear_analysis.recommendations.length > 0 && (
                <div className="wear-recommendations">
                  <h5>⚠️ Recommendations</h5>
                  <ul>
                    {analysis.wear_analysis.recommendations.map((rec, i) => (
                      <li key={i}>{rec}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
        )}

        {activeSection === "map" && analysis.memory_map && (
          <div className="ai-map">
            <h4>Memory Map</h4>
            <p className="map-size">Total: {formatBytes(analysis.memory_map.total_size)}</p>
            
            <div className="memory-map-visual">
              {analysis.memory_map.regions.map((region, i) => {
                const widthPercent = ((region.end - region.start) / analysis.memory_map!.total_size) * 100;
                if (widthPercent < 0.5) return null;
                
                return (
                  <div
                    key={i}
                    className="map-region"
                    style={{ 
                      width: `${Math.max(widthPercent, 1)}%`,
                      backgroundColor: region.color,
                    }}
                    title={`${region.name}\n0x${region.start.toString(16)} - 0x${region.end.toString(16)}\n${formatBytes(region.end - region.start)}`}
                    onClick={() => onPatternSelect?.(region.start)}
                  >
                    {widthPercent > 5 && (
                      <span className="region-label">{region.region_type}</span>
                    )}
                  </div>
                );
              })}
            </div>
            
            <div className="map-legend">
              {Array.from(new Set(analysis.memory_map.regions.map(r => r.region_type))).map((type, i) => {
                const region = analysis.memory_map!.regions.find(r => r.region_type === type);
                return (
                  <div key={i} className="legend-item">
                    <span 
                      className="legend-color" 
                      style={{ backgroundColor: region?.color || '#666' }}
                    />
                    <span>{type}</span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {activeSection === "recovery" && (
          <div className="ai-recovery">
            {analysis.recovery_suggestions.length === 0 ? (
              <p className="ai-empty">No recovery actions needed</p>
            ) : (
              <div className="ai-recovery-list">
                {analysis.recovery_suggestions.map((suggestion, i) => (
                  <div key={i} className="ai-recovery-item">
                    <div className="recovery-header">
                      <span className="recovery-priority">#{suggestion.priority}</span>
                      <span className="recovery-action">{suggestion.action}</span>
                      <span className="recovery-success">
                        {Math.round(suggestion.estimated_success * 100)}% success
                      </span>
                    </div>
                    <p className="recovery-desc">{suggestion.description}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeSection === "recommendations" && (
          <div className="ai-recommendations">
            {analysis.chip_recommendations.length === 0 ? (
              <p className="ai-empty">No recommendations</p>
            ) : (
              <div className="ai-recommendation-list">
                {analysis.chip_recommendations.map((rec, i) => (
                  <div key={i} className="ai-recommendation-item">
                    <div className="recommendation-header">
                      <span className="recommendation-category">{rec.category}</span>
                      <span className="recommendation-importance">
                        {"⭐".repeat(Math.min(Math.ceil(rec.importance / 2), 5))}
                      </span>
                    </div>
                    <h4 className="recommendation-title">{rec.title}</h4>
                    <p className="recommendation-desc">{rec.description}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
