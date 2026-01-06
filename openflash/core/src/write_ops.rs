//! Advanced Write Operations for OpenFlash v1.7
//! 
//! Provides full chip programming, bad block management, wear leveling,
//! incremental backup/restore, and chip-to-chip cloning.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Error Types
// ============================================================================

/// Write operation errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteError {
    /// Block is marked as bad
    BadBlock(u32),
    /// Program operation failed
    ProgramFailed { block: u32, page: u32 },
    /// Erase operation failed
    EraseFailed(u32),
    /// Verification failed after write
    VerifyFailed { block: u32, page: u32, offset: u32 },
    /// No spare blocks available for remapping
    NoSpareBlocks,
    /// Block wear limit exceeded
    WearLimitExceeded(u32),
    /// Invalid address
    InvalidAddress { block: u32, page: u32 },
    /// Data size mismatch
    DataSizeMismatch { expected: usize, actual: usize },
    /// Chip mismatch during clone
    ChipMismatch { source: String, target: String },
    /// Operation cancelled
    Cancelled,
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::BadBlock(b) => write!(f, "Bad block: {}", b),
            WriteError::ProgramFailed { block, page } => {
                write!(f, "Program failed at block {} page {}", block, page)
            }
            WriteError::EraseFailed(b) => write!(f, "Erase failed at block {}", b),
            WriteError::VerifyFailed { block, page, offset } => {
                write!(f, "Verify failed at block {} page {} offset {}", block, page, offset)
            }
            WriteError::NoSpareBlocks => write!(f, "No spare blocks available"),
            WriteError::WearLimitExceeded(b) => write!(f, "Wear limit exceeded for block {}", b),
            WriteError::InvalidAddress { block, page } => {
                write!(f, "Invalid address: block {} page {}", block, page)
            }
            WriteError::DataSizeMismatch { expected, actual } => {
                write!(f, "Data size mismatch: expected {} got {}", expected, actual)
            }
            WriteError::ChipMismatch { source, target } => {
                write!(f, "Chip mismatch: {} vs {}", source, target)
            }
            WriteError::Cancelled => write!(f, "Operation cancelled"),
            WriteError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for WriteError {}

pub type WriteResult<T> = Result<T, WriteError>;

// ============================================================================
// Bad Block Management
// ============================================================================

/// Bad block table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadBlockEntry {
    /// Original bad block number
    pub bad_block: u32,
    /// Replacement block number (if remapped)
    pub replacement: Option<u32>,
    /// Reason for marking as bad
    pub reason: BadBlockReason,
    /// Timestamp when marked bad
    pub timestamp: u64,
}

/// Reason for bad block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BadBlockReason {
    /// Factory marked bad
    Factory,
    /// Erase failure
    EraseFail,
    /// Program failure
    ProgramFail,
    /// Uncorrectable ECC error
    EccFail,
    /// Wear out
    WearOut,
    /// User marked
    UserMarked,
}

/// Bad block management table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadBlockTable {
    /// Map of bad blocks to their entries
    entries: HashMap<u32, BadBlockEntry>,
    /// Reserved blocks for remapping (spare area)
    spare_blocks: Vec<u32>,
    /// Next spare block index
    next_spare: usize,
    /// Total blocks in device
    total_blocks: u32,
    /// Reserved spare block count
    spare_count: u32,
}

impl BadBlockTable {
    /// Create new bad block table
    pub fn new(total_blocks: u32, spare_percentage: u8) -> Self {
        let spare_count = (total_blocks as u64 * spare_percentage as u64 / 100) as u32;
        let spare_start = total_blocks - spare_count;
        
        Self {
            entries: HashMap::new(),
            spare_blocks: (spare_start..total_blocks).collect(),
            next_spare: 0,
            total_blocks,
            spare_count,
        }
    }

    /// Check if block is bad
    pub fn is_bad(&self, block: u32) -> bool {
        self.entries.contains_key(&block)
    }

    /// Get replacement block for bad block
    pub fn get_replacement(&self, block: u32) -> Option<u32> {
        self.entries.get(&block).and_then(|e| e.replacement)
    }

    /// Mark block as bad and allocate replacement
    pub fn mark_bad(&mut self, block: u32, reason: BadBlockReason) -> WriteResult<Option<u32>> {
        if self.entries.contains_key(&block) {
            return Ok(self.entries[&block].replacement);
        }

        let replacement = self.allocate_spare()?;
        
        self.entries.insert(block, BadBlockEntry {
            bad_block: block,
            replacement,
            reason,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });

        Ok(replacement)
    }

    /// Allocate a spare block
    fn allocate_spare(&mut self) -> WriteResult<Option<u32>> {
        if self.next_spare >= self.spare_blocks.len() {
            return Err(WriteError::NoSpareBlocks);
        }

        let spare = self.spare_blocks[self.next_spare];
        self.next_spare += 1;
        Ok(Some(spare))
    }

    /// Get all bad blocks
    pub fn bad_blocks(&self) -> Vec<u32> {
        self.entries.keys().copied().collect()
    }

    /// Get bad block count
    pub fn bad_count(&self) -> usize {
        self.entries.len()
    }

    /// Get available spare count
    pub fn available_spares(&self) -> usize {
        self.spare_blocks.len() - self.next_spare
    }

    /// Translate logical block to physical block
    pub fn translate(&self, logical: u32) -> u32 {
        self.get_replacement(logical).unwrap_or(logical)
    }

    /// Scan for factory bad blocks from OOB markers
    pub fn scan_factory_bad_blocks(&mut self, oob_markers: &[(u32, u8)]) {
        for &(block, marker) in oob_markers {
            // Factory bad blocks typically have 0x00 in first OOB byte
            if marker != 0xFF {
                let _ = self.mark_bad(block, BadBlockReason::Factory);
            }
        }
    }
}


// ============================================================================
// Wear Leveling
// ============================================================================

/// Wear leveling statistics for a block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockWearInfo {
    /// Erase count for this block
    pub erase_count: u32,
    /// Program count for this block
    pub program_count: u32,
    /// Last access timestamp
    pub last_access: u64,
    /// Is this block considered "hot" (frequently written)
    pub is_hot: bool,
}

/// Wear leveling manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WearLevelingManager {
    /// Wear info per block
    block_info: HashMap<u32, BlockWearInfo>,
    /// Maximum allowed erase cycles
    max_erase_cycles: u32,
    /// Threshold for wear leveling trigger
    wear_threshold: u32,
    /// Total blocks
    total_blocks: u32,
}

impl WearLevelingManager {
    /// Create new wear leveling manager
    pub fn new(total_blocks: u32, max_erase_cycles: u32) -> Self {
        Self {
            block_info: HashMap::new(),
            max_erase_cycles,
            wear_threshold: max_erase_cycles / 10, // 10% threshold
            total_blocks,
        }
    }

    /// Record an erase operation
    pub fn record_erase(&mut self, block: u32) -> WriteResult<()> {
        let info = self.block_info.entry(block).or_default();
        info.erase_count += 1;
        info.last_access = current_timestamp();

        if info.erase_count >= self.max_erase_cycles {
            return Err(WriteError::WearLimitExceeded(block));
        }

        Ok(())
    }

    /// Record a program operation
    pub fn record_program(&mut self, block: u32) {
        let info = self.block_info.entry(block).or_default();
        info.program_count += 1;
        info.last_access = current_timestamp();
    }

    /// Get erase count for block
    pub fn get_erase_count(&self, block: u32) -> u32 {
        self.block_info.get(&block).map(|i| i.erase_count).unwrap_or(0)
    }

    /// Get average erase count
    pub fn average_erase_count(&self) -> f64 {
        if self.block_info.is_empty() {
            return 0.0;
        }
        let total: u64 = self.block_info.values().map(|i| i.erase_count as u64).sum();
        total as f64 / self.block_info.len() as f64
    }

    /// Find least worn block
    pub fn find_least_worn(&self) -> Option<u32> {
        self.block_info
            .iter()
            .min_by_key(|(_, info)| info.erase_count)
            .map(|(&block, _)| block)
    }

    /// Find most worn block
    pub fn find_most_worn(&self) -> Option<u32> {
        self.block_info
            .iter()
            .max_by_key(|(_, info)| info.erase_count)
            .map(|(&block, _)| block)
    }

    /// Check if wear leveling is needed
    pub fn needs_leveling(&self) -> bool {
        let min = self.block_info.values().map(|i| i.erase_count).min().unwrap_or(0);
        let max = self.block_info.values().map(|i| i.erase_count).max().unwrap_or(0);
        max - min > self.wear_threshold
    }

    /// Get blocks that should be moved for wear leveling
    pub fn get_leveling_candidates(&self) -> Vec<(u32, u32)> {
        let avg = self.average_erase_count();
        let mut hot_blocks: Vec<_> = self.block_info
            .iter()
            .filter(|(_, info)| info.erase_count as f64 > avg * 1.5)
            .map(|(&b, _)| b)
            .collect();
        
        let mut cold_blocks: Vec<_> = self.block_info
            .iter()
            .filter(|(_, info)| (info.erase_count as f64) < avg * 0.5)
            .map(|(&b, _)| b)
            .collect();

        hot_blocks.sort_by_key(|b| std::cmp::Reverse(self.get_erase_count(*b)));
        cold_blocks.sort_by_key(|b| self.get_erase_count(*b));

        hot_blocks.into_iter()
            .zip(cold_blocks.into_iter())
            .collect()
    }

    /// Get wear statistics
    pub fn get_statistics(&self) -> WearStatistics {
        let counts: Vec<u32> = self.block_info.values().map(|i| i.erase_count).collect();
        
        WearStatistics {
            total_blocks: self.total_blocks,
            tracked_blocks: self.block_info.len() as u32,
            min_erase_count: counts.iter().min().copied().unwrap_or(0),
            max_erase_count: counts.iter().max().copied().unwrap_or(0),
            avg_erase_count: self.average_erase_count(),
            max_allowed: self.max_erase_cycles,
            remaining_life_percent: self.remaining_life_percent(),
        }
    }

    /// Calculate remaining life percentage
    pub fn remaining_life_percent(&self) -> f64 {
        let max_count = self.block_info.values()
            .map(|i| i.erase_count)
            .max()
            .unwrap_or(0);
        
        if self.max_erase_cycles == 0 {
            return 100.0;
        }
        
        ((self.max_erase_cycles - max_count) as f64 / self.max_erase_cycles as f64) * 100.0
    }
}

/// Wear leveling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WearStatistics {
    pub total_blocks: u32,
    pub tracked_blocks: u32,
    pub min_erase_count: u32,
    pub max_erase_count: u32,
    pub avg_erase_count: f64,
    pub max_allowed: u32,
    pub remaining_life_percent: f64,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}


// ============================================================================
// Full Chip Programming
// ============================================================================

/// Programming options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramOptions {
    /// Verify after write
    pub verify: bool,
    /// Skip bad blocks
    pub skip_bad_blocks: bool,
    /// Use wear leveling
    pub wear_leveling: bool,
    /// Erase before program
    pub erase_before_program: bool,
    /// Number of retries on failure
    pub retry_count: u8,
    /// Progress callback interval (pages)
    pub progress_interval: u32,
}

impl Default for ProgramOptions {
    fn default() -> Self {
        Self {
            verify: true,
            skip_bad_blocks: true,
            wear_leveling: false,
            erase_before_program: true,
            retry_count: 3,
            progress_interval: 64,
        }
    }
}

/// Programming progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramProgress {
    /// Current block being programmed
    pub current_block: u32,
    /// Current page within block
    pub current_page: u32,
    /// Total blocks to program
    pub total_blocks: u32,
    /// Bytes written so far
    pub bytes_written: u64,
    /// Total bytes to write
    pub total_bytes: u64,
    /// Bad blocks encountered
    pub bad_blocks_skipped: u32,
    /// Verify errors (corrected by retry)
    pub verify_retries: u32,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u32>,
    /// Current operation
    pub operation: ProgramOperation,
}

/// Current programming operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramOperation {
    Erasing,
    Programming,
    Verifying,
    Idle,
}

impl ProgramProgress {
    /// Calculate completion percentage
    pub fn percent_complete(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_written as f64 / self.total_bytes as f64) * 100.0
    }
}

/// Chip programmer with verification
#[derive(Debug)]
pub struct ChipProgrammer {
    /// Page size in bytes
    page_size: u32,
    /// Pages per block
    pages_per_block: u32,
    /// Total blocks
    total_blocks: u32,
    /// OOB size per page
    oob_size: u32,
    /// Bad block table
    bbt: BadBlockTable,
    /// Wear leveling manager
    wear_manager: WearLevelingManager,
    /// Programming options
    options: ProgramOptions,
}

impl ChipProgrammer {
    /// Create new chip programmer
    pub fn new(
        page_size: u32,
        pages_per_block: u32,
        total_blocks: u32,
        oob_size: u32,
        max_erase_cycles: u32,
    ) -> Self {
        Self {
            page_size,
            pages_per_block,
            total_blocks,
            oob_size,
            bbt: BadBlockTable::new(total_blocks, 2), // 2% spare
            wear_manager: WearLevelingManager::new(total_blocks, max_erase_cycles),
            options: ProgramOptions::default(),
        }
    }

    /// Set programming options
    pub fn set_options(&mut self, options: ProgramOptions) {
        self.options = options;
    }

    /// Get bad block table reference
    pub fn bad_block_table(&self) -> &BadBlockTable {
        &self.bbt
    }

    /// Get mutable bad block table reference
    pub fn bad_block_table_mut(&mut self) -> &mut BadBlockTable {
        &mut self.bbt
    }

    /// Get wear manager reference
    pub fn wear_manager(&self) -> &WearLevelingManager {
        &self.wear_manager
    }

    /// Calculate block and page from linear address
    pub fn address_to_block_page(&self, address: u64) -> (u32, u32) {
        let page = (address / self.page_size as u64) as u32;
        let block = page / self.pages_per_block;
        let page_in_block = page % self.pages_per_block;
        (block, page_in_block)
    }

    /// Calculate linear address from block and page
    pub fn block_page_to_address(&self, block: u32, page: u32) -> u64 {
        let total_page = block * self.pages_per_block + page;
        total_page as u64 * self.page_size as u64
    }

    /// Prepare block for programming (erase if needed)
    pub fn prepare_block(&mut self, block: u32) -> WriteResult<u32> {
        // Check if block is bad
        if self.bbt.is_bad(block) {
            if self.options.skip_bad_blocks {
                if let Some(replacement) = self.bbt.get_replacement(block) {
                    return Ok(replacement);
                }
            }
            return Err(WriteError::BadBlock(block));
        }

        // Translate to physical block
        let physical_block = self.bbt.translate(block);

        // Record erase in wear manager
        if self.options.wear_leveling {
            self.wear_manager.record_erase(physical_block)?;
        }

        Ok(physical_block)
    }

    /// Verify programmed data
    pub fn verify_page(&self, expected: &[u8], actual: &[u8], block: u32, page: u32) -> WriteResult<()> {
        if expected.len() != actual.len() {
            return Err(WriteError::DataSizeMismatch {
                expected: expected.len(),
                actual: actual.len(),
            });
        }

        for (i, (&e, &a)) in expected.iter().zip(actual.iter()).enumerate() {
            if e != a {
                return Err(WriteError::VerifyFailed {
                    block,
                    page,
                    offset: i as u32,
                });
            }
        }

        Ok(())
    }

    /// Get chip capacity in bytes
    pub fn capacity(&self) -> u64 {
        self.total_blocks as u64 * self.pages_per_block as u64 * self.page_size as u64
    }

    /// Get usable capacity (excluding bad blocks and spare area)
    pub fn usable_capacity(&self) -> u64 {
        let usable_blocks = self.total_blocks - self.bbt.bad_count() as u32 - self.bbt.spare_count;
        usable_blocks as u64 * self.pages_per_block as u64 * self.page_size as u64
    }
}


// ============================================================================
// Incremental Backup/Restore
// ============================================================================

/// Block change tracking for incremental backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTracker {
    /// Set of modified blocks since last backup
    modified_blocks: HashSet<u32>,
    /// Block checksums from last backup
    block_checksums: HashMap<u32, u64>,
    /// Last backup timestamp
    last_backup: u64,
    /// Total blocks
    total_blocks: u32,
}

impl ChangeTracker {
    /// Create new change tracker
    pub fn new(total_blocks: u32) -> Self {
        Self {
            modified_blocks: HashSet::new(),
            block_checksums: HashMap::new(),
            last_backup: 0,
            total_blocks,
        }
    }

    /// Mark block as modified
    pub fn mark_modified(&mut self, block: u32) {
        self.modified_blocks.insert(block);
    }

    /// Get modified blocks since last backup
    pub fn get_modified_blocks(&self) -> Vec<u32> {
        let mut blocks: Vec<_> = self.modified_blocks.iter().copied().collect();
        blocks.sort();
        blocks
    }

    /// Update checksum for block
    pub fn update_checksum(&mut self, block: u32, checksum: u64) {
        self.block_checksums.insert(block, checksum);
    }

    /// Get stored checksum for block
    pub fn get_checksum(&self, block: u32) -> Option<u64> {
        self.block_checksums.get(&block).copied()
    }

    /// Clear modified blocks after backup
    pub fn clear_modified(&mut self) {
        self.modified_blocks.clear();
        self.last_backup = current_timestamp();
    }

    /// Check if block has changed by comparing checksums
    pub fn has_changed(&self, block: u32, current_checksum: u64) -> bool {
        match self.block_checksums.get(&block) {
            Some(&stored) => stored != current_checksum,
            None => true, // No previous checksum, assume changed
        }
    }

    /// Get number of modified blocks
    pub fn modified_count(&self) -> usize {
        self.modified_blocks.len()
    }

    /// Calculate simple checksum (FNV-1a)
    pub fn calculate_checksum(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Incremental backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup creation timestamp
    pub timestamp: u64,
    /// Source chip identifier
    pub chip_id: String,
    /// Total chip size
    pub total_size: u64,
    /// Page size
    pub page_size: u32,
    /// Block size (pages per block)
    pub block_size: u32,
    /// Is this a full backup?
    pub is_full: bool,
    /// Parent backup ID (for incremental)
    pub parent_id: Option<String>,
    /// Blocks included in this backup
    pub included_blocks: Vec<u32>,
    /// Bad blocks at time of backup
    pub bad_blocks: Vec<u32>,
    /// Block checksums
    pub checksums: HashMap<u32, u64>,
}

impl BackupMetadata {
    /// Create metadata for full backup
    pub fn new_full(chip_id: String, total_size: u64, page_size: u32, block_size: u32) -> Self {
        Self {
            timestamp: current_timestamp(),
            chip_id,
            total_size,
            page_size,
            block_size,
            is_full: true,
            parent_id: None,
            included_blocks: Vec::new(),
            bad_blocks: Vec::new(),
            checksums: HashMap::new(),
        }
    }

    /// Create metadata for incremental backup
    pub fn new_incremental(
        chip_id: String,
        total_size: u64,
        page_size: u32,
        block_size: u32,
        parent_id: String,
    ) -> Self {
        Self {
            timestamp: current_timestamp(),
            chip_id,
            total_size,
            page_size,
            block_size,
            is_full: false,
            parent_id: Some(parent_id),
            included_blocks: Vec::new(),
            bad_blocks: Vec::new(),
            checksums: HashMap::new(),
        }
    }

    /// Generate backup ID
    pub fn generate_id(&self) -> String {
        format!("{}_{}", self.chip_id, self.timestamp)
    }

    /// Get backup size estimate
    pub fn estimated_size(&self) -> u64 {
        self.included_blocks.len() as u64 
            * self.block_size as u64 
            * self.page_size as u64
    }
}


// ============================================================================
// Chip-to-Chip Cloning
// ============================================================================

/// Clone operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneMode {
    /// Exact copy including bad block markers
    Exact,
    /// Skip bad blocks on source, remap on target
    SkipBadBlocks,
    /// Intelligent clone with wear leveling
    WearAware,
}

/// Clone operation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneOptions {
    /// Clone mode
    pub mode: CloneMode,
    /// Verify after each block
    pub verify: bool,
    /// Allow different chip models (same capacity)
    pub allow_different_models: bool,
    /// Preserve OOB data
    pub preserve_oob: bool,
    /// Number of retries on error
    pub retry_count: u8,
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self {
            mode: CloneMode::SkipBadBlocks,
            verify: true,
            allow_different_models: false,
            preserve_oob: true,
            retry_count: 3,
        }
    }
}

/// Clone operation progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneProgress {
    /// Current block being cloned
    pub current_block: u32,
    /// Total blocks to clone
    pub total_blocks: u32,
    /// Bytes cloned so far
    pub bytes_cloned: u64,
    /// Total bytes to clone
    pub total_bytes: u64,
    /// Source bad blocks skipped
    pub source_bad_blocks: u32,
    /// Target bad blocks remapped
    pub target_remapped: u32,
    /// Verify errors (corrected)
    pub verify_errors: u32,
    /// Current phase
    pub phase: ClonePhase,
}

/// Clone operation phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClonePhase {
    /// Scanning source chip
    ScanningSource,
    /// Scanning target chip
    ScanningTarget,
    /// Erasing target
    ErasingTarget,
    /// Copying data
    Copying,
    /// Verifying
    Verifying,
    /// Complete
    Complete,
}

impl CloneProgress {
    /// Calculate completion percentage
    pub fn percent_complete(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_cloned as f64 / self.total_bytes as f64) * 100.0
    }
}

/// Chip clone manager
#[derive(Debug)]
pub struct ChipCloner {
    /// Clone options
    options: CloneOptions,
    /// Source chip info
    source_page_size: u32,
    source_pages_per_block: u32,
    source_total_blocks: u32,
    /// Target chip info
    target_page_size: u32,
    target_pages_per_block: u32,
    target_total_blocks: u32,
}

impl ChipCloner {
    /// Create new chip cloner
    pub fn new(
        source_page_size: u32,
        source_pages_per_block: u32,
        source_total_blocks: u32,
        target_page_size: u32,
        target_pages_per_block: u32,
        target_total_blocks: u32,
    ) -> WriteResult<Self> {
        // Validate compatibility
        let source_capacity = source_page_size as u64 
            * source_pages_per_block as u64 
            * source_total_blocks as u64;
        let target_capacity = target_page_size as u64 
            * target_pages_per_block as u64 
            * target_total_blocks as u64;

        if target_capacity < source_capacity {
            return Err(WriteError::ChipMismatch {
                source: format!("{}MB", source_capacity / 1024 / 1024),
                target: format!("{}MB", target_capacity / 1024 / 1024),
            });
        }

        Ok(Self {
            options: CloneOptions::default(),
            source_page_size,
            source_pages_per_block,
            source_total_blocks,
            target_page_size,
            target_pages_per_block,
            target_total_blocks,
        })
    }

    /// Set clone options
    pub fn set_options(&mut self, options: CloneOptions) {
        self.options = options;
    }

    /// Check if chips are compatible for cloning
    pub fn check_compatibility(&self) -> WriteResult<()> {
        if !self.options.allow_different_models {
            if self.source_page_size != self.target_page_size {
                return Err(WriteError::ChipMismatch {
                    source: format!("page_size={}", self.source_page_size),
                    target: format!("page_size={}", self.target_page_size),
                });
            }
            if self.source_pages_per_block != self.target_pages_per_block {
                return Err(WriteError::ChipMismatch {
                    source: format!("pages_per_block={}", self.source_pages_per_block),
                    target: format!("pages_per_block={}", self.target_pages_per_block),
                });
            }
        }
        Ok(())
    }

    /// Get source capacity
    pub fn source_capacity(&self) -> u64 {
        self.source_page_size as u64 
            * self.source_pages_per_block as u64 
            * self.source_total_blocks as u64
    }

    /// Get target capacity
    pub fn target_capacity(&self) -> u64 {
        self.target_page_size as u64 
            * self.target_pages_per_block as u64 
            * self.target_total_blocks as u64
    }

    /// Create block mapping for clone operation
    pub fn create_block_mapping(
        &self,
        source_bad_blocks: &[u32],
        target_bad_blocks: &[u32],
    ) -> HashMap<u32, u32> {
        let mut mapping = HashMap::new();
        let source_bad: HashSet<_> = source_bad_blocks.iter().copied().collect();
        let target_bad: HashSet<_> = target_bad_blocks.iter().copied().collect();

        let mut target_block = 0u32;
        
        for source_block in 0..self.source_total_blocks {
            if self.options.mode == CloneMode::SkipBadBlocks && source_bad.contains(&source_block) {
                continue;
            }

            // Find next good target block
            while target_block < self.target_total_blocks && target_bad.contains(&target_block) {
                target_block += 1;
            }

            if target_block < self.target_total_blocks {
                mapping.insert(source_block, target_block);
                target_block += 1;
            }
        }

        mapping
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_block_table_creation() {
        let bbt = BadBlockTable::new(1024, 2);
        assert_eq!(bbt.bad_count(), 0);
        assert!(bbt.available_spares() > 0);
    }

    #[test]
    fn test_mark_bad_block() {
        let mut bbt = BadBlockTable::new(1024, 2);
        let result = bbt.mark_bad(100, BadBlockReason::EraseFail);
        assert!(result.is_ok());
        assert!(bbt.is_bad(100));
        assert!(bbt.get_replacement(100).is_some());
    }

    #[test]
    fn test_bad_block_translation() {
        let mut bbt = BadBlockTable::new(1024, 2);
        let _ = bbt.mark_bad(50, BadBlockReason::Factory);
        
        // Good block should translate to itself
        assert_eq!(bbt.translate(10), 10);
        
        // Bad block should translate to replacement
        let replacement = bbt.get_replacement(50).unwrap();
        assert_eq!(bbt.translate(50), replacement);
    }

    #[test]
    fn test_wear_leveling_manager() {
        let mut wm = WearLevelingManager::new(1024, 100000);
        
        // Record some erases
        for _ in 0..100 {
            wm.record_erase(10).unwrap();
        }
        
        assert_eq!(wm.get_erase_count(10), 100);
        assert_eq!(wm.get_erase_count(20), 0);
    }

    #[test]
    fn test_wear_limit_exceeded() {
        let mut wm = WearLevelingManager::new(1024, 100);
        
        for _ in 0..100 {
            let _ = wm.record_erase(5);
        }
        
        let result = wm.record_erase(5);
        assert!(matches!(result, Err(WriteError::WearLimitExceeded(5))));
    }

    #[test]
    fn test_wear_statistics() {
        let mut wm = WearLevelingManager::new(1024, 100000);
        
        wm.record_erase(0).unwrap();
        wm.record_erase(0).unwrap();
        wm.record_erase(1).unwrap();
        
        let stats = wm.get_statistics();
        assert_eq!(stats.min_erase_count, 1);
        assert_eq!(stats.max_erase_count, 2);
        assert!(stats.remaining_life_percent > 99.0);
    }

    #[test]
    fn test_change_tracker() {
        let mut tracker = ChangeTracker::new(1024);
        
        tracker.mark_modified(10);
        tracker.mark_modified(20);
        tracker.mark_modified(10); // Duplicate
        
        let modified = tracker.get_modified_blocks();
        assert_eq!(modified.len(), 2);
        assert!(modified.contains(&10));
        assert!(modified.contains(&20));
    }

    #[test]
    fn test_checksum_calculation() {
        let data1 = vec![0u8; 2048];
        let data2 = vec![1u8; 2048];
        
        let cs1 = ChangeTracker::calculate_checksum(&data1);
        let cs2 = ChangeTracker::calculate_checksum(&data2);
        
        assert_ne!(cs1, cs2);
        
        // Same data should produce same checksum
        let cs1_again = ChangeTracker::calculate_checksum(&data1);
        assert_eq!(cs1, cs1_again);
    }

    #[test]
    fn test_chip_programmer_address_conversion() {
        let programmer = ChipProgrammer::new(2048, 64, 1024, 64, 100000);
        
        // Block 10, page 5
        let addr = programmer.block_page_to_address(10, 5);
        let (block, page) = programmer.address_to_block_page(addr);
        
        assert_eq!(block, 10);
        assert_eq!(page, 5);
    }

    #[test]
    fn test_chip_programmer_capacity() {
        let programmer = ChipProgrammer::new(2048, 64, 1024, 64, 100000);
        
        // 1024 blocks * 64 pages * 2048 bytes = 128MB
        assert_eq!(programmer.capacity(), 128 * 1024 * 1024);
    }

    #[test]
    fn test_backup_metadata() {
        let meta = BackupMetadata::new_full(
            "TEST_CHIP".to_string(),
            128 * 1024 * 1024,
            2048,
            64,
        );
        
        assert!(meta.is_full);
        assert!(meta.parent_id.is_none());
        assert!(!meta.chip_id.is_empty());
    }

    #[test]
    fn test_incremental_backup_metadata() {
        let meta = BackupMetadata::new_incremental(
            "TEST_CHIP".to_string(),
            128 * 1024 * 1024,
            2048,
            64,
            "parent_123".to_string(),
        );
        
        assert!(!meta.is_full);
        assert_eq!(meta.parent_id, Some("parent_123".to_string()));
    }

    #[test]
    fn test_chip_cloner_compatibility() {
        // Same chips - should be compatible
        let cloner = ChipCloner::new(2048, 64, 1024, 2048, 64, 1024);
        assert!(cloner.is_ok());
        
        // Target smaller than source - should fail
        let cloner = ChipCloner::new(2048, 64, 1024, 2048, 64, 512);
        assert!(cloner.is_err());
    }

    #[test]
    fn test_clone_block_mapping() {
        let cloner = ChipCloner::new(2048, 64, 100, 2048, 64, 100).unwrap();
        
        let source_bad = vec![5, 10];
        let target_bad = vec![7, 15];
        
        let mapping = cloner.create_block_mapping(&source_bad, &target_bad);
        
        // Source blocks 5 and 10 should not be in mapping (skipped)
        assert!(!mapping.contains_key(&5));
        assert!(!mapping.contains_key(&10));
        
        // Target blocks 7 and 15 should not be targets
        assert!(!mapping.values().any(|&v| v == 7 || v == 15));
    }

    #[test]
    fn test_clone_progress() {
        let progress = CloneProgress {
            current_block: 50,
            total_blocks: 100,
            bytes_cloned: 50 * 64 * 2048,
            total_bytes: 100 * 64 * 2048,
            source_bad_blocks: 2,
            target_remapped: 1,
            verify_errors: 0,
            phase: ClonePhase::Copying,
        };
        
        assert!((progress.percent_complete() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_program_options_default() {
        let opts = ProgramOptions::default();
        assert!(opts.verify);
        assert!(opts.skip_bad_blocks);
        assert!(opts.erase_before_program);
        assert_eq!(opts.retry_count, 3);
    }

    #[test]
    fn test_verify_page_success() {
        let programmer = ChipProgrammer::new(2048, 64, 1024, 64, 100000);
        let data = vec![0xAB; 2048];
        
        let result = programmer.verify_page(&data, &data, 0, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_page_failure() {
        let programmer = ChipProgrammer::new(2048, 64, 1024, 64, 100000);
        let expected = vec![0xAB; 2048];
        let mut actual = vec![0xAB; 2048];
        actual[100] = 0xCD; // Introduce error
        
        let result = programmer.verify_page(&expected, &actual, 5, 10);
        assert!(matches!(result, Err(WriteError::VerifyFailed { block: 5, page: 10, offset: 100 })));
    }
}
