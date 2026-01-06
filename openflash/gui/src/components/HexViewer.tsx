import { useState, useCallback, useRef, useEffect } from "react";

interface HexViewerProps {
  data: Uint8Array;
  bytesPerRow?: number;
  pageSize?: number;
  highlights?: { start: number; end: number; color: string; label?: string }[];
}

export function HexViewer({ 
  data, 
  bytesPerRow = 16, 
  pageSize = 512,
  highlights = []
}: HexViewerProps) {
  const [currentPage, setCurrentPage] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<number[]>([]);
  const [currentSearchIndex, setCurrentSearchIndex] = useState(0);
  const [goToOffset, setGoToOffset] = useState("");
  const [selectedByte, setSelectedByte] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalPages = Math.ceil(data.length / pageSize);
  const startOffset = currentPage * pageSize;
  const endOffset = Math.min(startOffset + pageSize, data.length);
  const pageData = data.slice(startOffset, endOffset);
  const rows = Math.ceil(pageData.length / bytesPerRow);

  // Search functionality
  const handleSearch = useCallback(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }

    const results: number[] = [];
    const query = searchQuery.toLowerCase().replace(/\s/g, "");
    
    // Hex search
    if (/^[0-9a-f]+$/i.test(query) && query.length % 2 === 0) {
      const searchBytes = [];
      for (let i = 0; i < query.length; i += 2) {
        searchBytes.push(parseInt(query.substr(i, 2), 16));
      }
      
      for (let i = 0; i <= data.length - searchBytes.length; i++) {
        let match = true;
        for (let j = 0; j < searchBytes.length; j++) {
          if (data[i + j] !== searchBytes[j]) {
            match = false;
            break;
          }
        }
        if (match) results.push(i);
      }
    } else {
      // ASCII search
      const encoder = new TextEncoder();
      const searchBytes = encoder.encode(searchQuery);
      
      for (let i = 0; i <= data.length - searchBytes.length; i++) {
        let match = true;
        for (let j = 0; j < searchBytes.length; j++) {
          if (data[i + j] !== searchBytes[j]) {
            match = false;
            break;
          }
        }
        if (match) results.push(i);
      }
    }
    
    setSearchResults(results);
    setCurrentSearchIndex(0);
    
    if (results.length > 0) {
      goToByteOffset(results[0]);
    }
  }, [searchQuery, data]);

  const goToByteOffset = useCallback((offset: number) => {
    const page = Math.floor(offset / pageSize);
    setCurrentPage(page);
    setSelectedByte(offset);
  }, [pageSize]);

  const handleGoTo = useCallback(() => {
    const offset = parseInt(goToOffset, 16);
    if (!isNaN(offset) && offset >= 0 && offset < data.length) {
      goToByteOffset(offset);
      setGoToOffset("");
    }
  }, [goToOffset, data.length, goToByteOffset]);

  const nextSearchResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const nextIndex = (currentSearchIndex + 1) % searchResults.length;
    setCurrentSearchIndex(nextIndex);
    goToByteOffset(searchResults[nextIndex]);
  }, [searchResults, currentSearchIndex, goToByteOffset]);

  const prevSearchResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const prevIndex = (currentSearchIndex - 1 + searchResults.length) % searchResults.length;
    setCurrentSearchIndex(prevIndex);
    goToByteOffset(searchResults[prevIndex]);
  }, [searchResults, currentSearchIndex, goToByteOffset]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowLeft" && currentPage > 0) {
        setCurrentPage(p => p - 1);
      } else if (e.key === "ArrowRight" && currentPage < totalPages - 1) {
        setCurrentPage(p => p + 1);
      } else if (e.key === "Home") {
        setCurrentPage(0);
      } else if (e.key === "End") {
        setCurrentPage(totalPages - 1);
      } else if (e.key === "F3" || (e.ctrlKey && e.key === "g")) {
        e.preventDefault();
        nextSearchResult();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentPage, totalPages, nextSearchResult]);

  const isHighlighted = useCallback((offset: number) => {
    return highlights.find(h => offset >= h.start && offset < h.end);
  }, [highlights]);

  const isSearchMatch = useCallback((offset: number) => {
    return searchResults.includes(offset);
  }, [searchResults]);

  const formatByte = (byte: number) => byte.toString(16).padStart(2, "0").toUpperCase();
  
  const formatAscii = (byte: number) => {
    if (byte >= 32 && byte <= 126) return String.fromCharCode(byte);
    return "·";
  };

  return (
    <div className="hex-viewer" ref={containerRef}>
      {/* Toolbar */}
      <div className="hex-toolbar">
        <div className="hex-search">
          <input
            type="text"
            placeholder="Search hex or ASCII..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
          <button onClick={handleSearch} className="icon-btn" title="Search">
            🔍
          </button>
          {searchResults.length > 0 && (
            <div className="search-nav">
              <button onClick={prevSearchResult} className="icon-btn" title="Previous">
                ◀
              </button>
              <span>{currentSearchIndex + 1} / {searchResults.length}</span>
              <button onClick={nextSearchResult} className="icon-btn" title="Next">
                ▶
              </button>
            </div>
          )}
        </div>
        
        <div className="hex-goto">
          <input
            type="text"
            placeholder="Go to offset..."
            value={goToOffset}
            onChange={(e) => setGoToOffset(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGoTo()}
          />
          <button onClick={handleGoTo} className="icon-btn" title="Go">
            ↵
          </button>
        </div>

        <div className="hex-info">
          <span className="hex-size">{(data.length / 1024).toFixed(1)} KB</span>
          {selectedByte !== null && (
            <span className="hex-selected">
              Selected: 0x{selectedByte.toString(16).toUpperCase().padStart(8, "0")}
            </span>
          )}
        </div>
      </div>

      {/* Hex Content */}
      <div className="hex-content">
        <div className="hex-header">
          <span className="hex-offset-header">Offset</span>
          <span className="hex-bytes-header">
            {Array.from({ length: bytesPerRow }, (_, i) => 
              i.toString(16).toUpperCase().padStart(2, "0")
            ).join(" ")}
          </span>
          <span className="hex-ascii-header">ASCII</span>
        </div>

        <div className="hex-rows">
          {Array.from({ length: rows }, (_, rowIndex) => {
            const rowOffset = startOffset + rowIndex * bytesPerRow;
            const rowBytes = pageData.slice(
              rowIndex * bytesPerRow,
              (rowIndex + 1) * bytesPerRow
            );

            return (
              <div key={rowIndex} className="hex-row">
                <span className="hex-offset">
                  {rowOffset.toString(16).toUpperCase().padStart(8, "0")}
                </span>
                <span className="hex-bytes">
                  {Array.from(rowBytes).map((byte, i) => {
                    const byteOffset = rowOffset + i;
                    const highlight = isHighlighted(byteOffset);
                    const isMatch = isSearchMatch(byteOffset);
                    const isSelected = selectedByte === byteOffset;
                    
                    return (
                      <span
                        key={i}
                        className={`hex-byte ${highlight ? "highlighted" : ""} ${isMatch ? "search-match" : ""} ${isSelected ? "selected" : ""}`}
                        style={highlight ? { backgroundColor: highlight.color } : undefined}
                        onClick={() => setSelectedByte(byteOffset)}
                        title={highlight?.label}
                      >
                        {formatByte(byte)}
                      </span>
                    );
                  })}
                  {rowBytes.length < bytesPerRow && (
                    <span className="hex-padding">
                      {"   ".repeat(bytesPerRow - rowBytes.length)}
                    </span>
                  )}
                </span>
                <span className="hex-ascii">
                  {Array.from(rowBytes).map((byte, i) => {
                    const byteOffset = rowOffset + i;
                    const highlight = isHighlighted(byteOffset);
                    const isMatch = isSearchMatch(byteOffset);
                    const isSelected = selectedByte === byteOffset;
                    
                    return (
                      <span
                        key={i}
                        className={`ascii-char ${highlight ? "highlighted" : ""} ${isMatch ? "search-match" : ""} ${isSelected ? "selected" : ""}`}
                        style={highlight ? { backgroundColor: highlight.color } : undefined}
                        onClick={() => setSelectedByte(byteOffset)}
                      >
                        {formatAscii(byte)}
                      </span>
                    );
                  })}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Pagination */}
      <div className="hex-pagination">
        <button 
          onClick={() => setCurrentPage(0)} 
          disabled={currentPage === 0}
          className="icon-btn"
          title="First page"
        >
          ⏮
        </button>
        <button 
          onClick={() => setCurrentPage(p => Math.max(0, p - 1))} 
          disabled={currentPage === 0}
          className="icon-btn"
          title="Previous page"
        >
          ◀
        </button>
        
        <div className="page-input">
          <input
            type="number"
            min={1}
            max={totalPages}
            value={currentPage + 1}
            onChange={(e) => {
              const page = parseInt(e.target.value) - 1;
              if (page >= 0 && page < totalPages) {
                setCurrentPage(page);
              }
            }}
          />
          <span>/ {totalPages.toLocaleString()}</span>
        </div>

        <button 
          onClick={() => setCurrentPage(p => Math.min(totalPages - 1, p + 1))} 
          disabled={currentPage >= totalPages - 1}
          className="icon-btn"
          title="Next page"
        >
          ▶
        </button>
        <button 
          onClick={() => setCurrentPage(totalPages - 1)} 
          disabled={currentPage >= totalPages - 1}
          className="icon-btn"
          title="Last page"
        >
          ⏭
        </button>

        <div className="page-slider">
          <input
            type="range"
            min={0}
            max={totalPages - 1}
            value={currentPage}
            onChange={(e) => setCurrentPage(parseInt(e.target.value))}
          />
        </div>
      </div>
    </div>
  );
}

export default HexViewer;
