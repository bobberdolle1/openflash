import { useMemo, useState, useCallback } from "react";

interface BitmapViewProps {
  data: Uint8Array;
  width?: number;
  pageSize?: number;
  onPageSelect?: (pageIndex: number) => void;
}

export function BitmapView({ 
  data, 
  width = 256, 
  pageSize = 2048,
  onPageSelect 
}: BitmapViewProps) {
  const [zoom, setZoom] = useState(1);
  const [hoveredPage, setHoveredPage] = useState<number | null>(null);
  const [selectedPage, setSelectedPage] = useState<number | null>(null);

  const totalPages = Math.ceil(data.length / pageSize);
  const height = Math.ceil(totalPages / width);

  const pageStats = useMemo(() => {
    const stats: { isEmpty: boolean; entropy: number; hasData: boolean; isZero: boolean }[] = [];
    
    for (let page = 0; page < totalPages; page++) {
      const pageStart = page * pageSize;
      const pageEnd = Math.min(pageStart + pageSize, data.length);
      const pageData = data.slice(pageStart, pageEnd);
      stats.push(analyzePageData(pageData));
    }
    
    return stats;
  }, [data, pageSize, totalPages]);

  const imageDataUrl = useMemo(() => {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return "";

    const imgData = ctx.createImageData(width, height);
    const pixels = imgData.data;

    for (let page = 0; page < totalPages; page++) {
      const x = page % width;
      const y = Math.floor(page / width);
      const pixelIndex = (y * width + x) * 4;
      const stat = pageStats[page];

      let r = 0, g = 0, b = 0;

      if (stat.isEmpty) {
        // Empty page (0xFF) - very light
        r = g = b = 245;
      } else if (stat.isZero) {
        // All zeros - could be bad block marker - red tint
        r = 255; g = 120; b = 120;
      } else {
        // Data page - color by entropy
        const normalizedEntropy = stat.entropy / 8;
        
        if (normalizedEntropy < 0.25) {
          // Very low entropy - deep blue
          r = 59; g = 130; b = 246;
        } else if (normalizedEntropy < 0.5) {
          // Low-medium entropy - cyan/teal
          r = 20; g = 184; b = 166;
        } else if (normalizedEntropy < 0.7) {
          // Medium entropy - green
          r = 34; g = 197; b = 94;
        } else if (normalizedEntropy < 0.85) {
          // High entropy - yellow/orange
          r = 245; g = 158; b = 11;
        } else {
          // Very high entropy - purple (likely encrypted/compressed)
          r = 168; g = 85; b = 247;
        }
      }

      pixels[pixelIndex] = r;
      pixels[pixelIndex + 1] = g;
      pixels[pixelIndex + 2] = b;
      pixels[pixelIndex + 3] = 255;
    }

    // Fill remaining pixels
    for (let i = totalPages; i < width * height; i++) {
      const pixelIndex = i * 4;
      pixels[pixelIndex] = 20;
      pixels[pixelIndex + 1] = 20;
      pixels[pixelIndex + 2] = 30;
      pixels[pixelIndex + 3] = 255;
    }

    ctx.putImageData(imgData, 0, 0);
    return canvas.toDataURL();
  }, [pageStats, width, height, totalPages]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    const rect = img.getBoundingClientRect();
    const scaleX = width / rect.width;
    const scaleY = height / rect.height;
    const x = Math.floor((e.clientX - rect.left) * scaleX);
    const y = Math.floor((e.clientY - rect.top) * scaleY);
    const pageIndex = y * width + x;
    
    if (pageIndex >= 0 && pageIndex < totalPages) {
      setHoveredPage(pageIndex);
    } else {
      setHoveredPage(null);
    }
  }, [width, height, totalPages]);

  const handleClick = useCallback((e: React.MouseEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    const rect = img.getBoundingClientRect();
    const scaleX = width / rect.width;
    const scaleY = height / rect.height;
    const x = Math.floor((e.clientX - rect.left) * scaleX);
    const y = Math.floor((e.clientY - rect.top) * scaleY);
    const pageIndex = y * width + x;
    
    if (pageIndex >= 0 && pageIndex < totalPages) {
      setSelectedPage(pageIndex);
      onPageSelect?.(pageIndex);
    }
  }, [width, height, totalPages, onPageSelect]);

  const hoveredStats = hoveredPage !== null ? pageStats[hoveredPage] : null;
  const selectedStats = selectedPage !== null ? pageStats[selectedPage] : null;

  // Calculate summary statistics
  const summary = useMemo(() => {
    let empty = 0, low = 0, medium = 0, high = 0, veryHigh = 0, bad = 0;
    
    for (const stat of pageStats) {
      if (stat.isEmpty) empty++;
      else if (stat.isZero) bad++;
      else {
        const e = stat.entropy / 8;
        if (e < 0.25) low++;
        else if (e < 0.5) low++;
        else if (e < 0.7) medium++;
        else if (e < 0.85) high++;
        else veryHigh++;
      }
    }
    
    return { empty, low, medium, high, veryHigh, bad, total: totalPages };
  }, [pageStats, totalPages]);

  return (
    <div className="bitmap-view">
      {/* Header with stats */}
      <div className="bitmap-header">
        <div className="bitmap-stats">
          <div className="stat">
            <span className="stat-value">{totalPages.toLocaleString()}</span>
            <span className="stat-label">Pages</span>
          </div>
          <div className="stat">
            <span className="stat-value">{width} × {height}</span>
            <span className="stat-label">Resolution</span>
          </div>
          <div className="stat">
            <span className="stat-value">{((data.length / 1024 / 1024)).toFixed(1)} MB</span>
            <span className="stat-label">Size</span>
          </div>
        </div>
        
        <div className="bitmap-zoom">
          <button onClick={() => setZoom(z => Math.max(0.5, z - 0.25))} className="icon-btn">−</button>
          <span>{Math.round(zoom * 100)}%</span>
          <button onClick={() => setZoom(z => Math.min(4, z + 0.25))} className="icon-btn">+</button>
          <button onClick={() => setZoom(1)} className="icon-btn" title="Reset">⟲</button>
        </div>
      </div>

      {/* Main content */}
      <div className="bitmap-content">
        <div className="bitmap-canvas-container" style={{ transform: `scale(${zoom})`, transformOrigin: 'top left' }}>
          <img 
            src={imageDataUrl} 
            alt="NAND Bitmap" 
            className="bitmap-image"
            onMouseMove={handleMouseMove}
            onMouseLeave={() => setHoveredPage(null)}
            onClick={handleClick}
          />
        </div>
      </div>

      {/* Footer with legend and info */}
      <div className="bitmap-footer">
        <div className="bitmap-legend">
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#f5f5f5" }} />
            <span>Empty</span>
            <span className="legend-count">{summary.empty}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#3b82f6" }} />
            <span>Low entropy</span>
            <span className="legend-count">{summary.low}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#22c55e" }} />
            <span>Medium</span>
            <span className="legend-count">{summary.medium}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#f59e0b" }} />
            <span>High</span>
            <span className="legend-count">{summary.high}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#a855f7" }} />
            <span>Very high</span>
            <span className="legend-count">{summary.veryHigh}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color" style={{ background: "#ff7878" }} />
            <span>Bad/Zero</span>
            <span className="legend-count">{summary.bad}</span>
          </div>
        </div>

        {/* Hover/Selected info */}
        <div className="bitmap-info">
          {hoveredStats && hoveredPage !== null && (
            <div className="page-info hover">
              <span className="page-label">Hover:</span>
              <span className="page-num">Page {hoveredPage.toLocaleString()}</span>
              <span className="page-offset">@ 0x{(hoveredPage * pageSize).toString(16).toUpperCase()}</span>
              <span className="page-entropy">
                {hoveredStats.isEmpty ? "Empty" : 
                 hoveredStats.isZero ? "Zero" : 
                 `Entropy: ${hoveredStats.entropy.toFixed(2)}`}
              </span>
            </div>
          )}
          {selectedStats && selectedPage !== null && (
            <div className="page-info selected">
              <span className="page-label">Selected:</span>
              <span className="page-num">Page {selectedPage.toLocaleString()}</span>
              <span className="page-offset">@ 0x{(selectedPage * pageSize).toString(16).toUpperCase()}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function analyzePageData(data: Uint8Array): { isEmpty: boolean; entropy: number; hasData: boolean; isZero: boolean } {
  if (data.length === 0) {
    return { isEmpty: true, entropy: 0, hasData: false, isZero: false };
  }

  let allFF = true;
  let all00 = true;
  const counts = new Array(256).fill(0);

  for (const byte of data) {
    counts[byte]++;
    if (byte !== 0xFF) allFF = false;
    if (byte !== 0x00) all00 = false;
  }

  if (allFF) {
    return { isEmpty: true, entropy: 0, hasData: false, isZero: false };
  }

  if (all00) {
    return { isEmpty: false, entropy: 0, hasData: false, isZero: true };
  }

  // Calculate Shannon entropy
  let entropy = 0;
  const len = data.length;
  for (const count of counts) {
    if (count > 0) {
      const p = count / len;
      entropy -= p * Math.log2(p);
    }
  }

  return { isEmpty: false, entropy, hasData: true, isZero: false };
}

export default BitmapView;
