//! Lua Sandbox Configuration
//!
//! Controls resource limits and security for Lua scripts.

/// Sandbox configuration for Lua scripts
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum memory per script in bytes (default: 1MB)
    pub memory_limit: Option<usize>,

    /// Maximum execution time per script in milliseconds (default: 10ms)
    pub timeout_ms: u64,

    /// Whether to allow file system access
    pub allow_filesystem: bool,

    /// Whether to allow network access
    pub allow_network: bool,

    /// Whether to allow OS execution
    pub allow_os: bool,

    /// Maximum recursion depth
    pub max_recursion: u32,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: Some(1024 * 1024), // 1MB
            timeout_ms: 10,
            allow_filesystem: false,
            allow_network: false,
            allow_os: false,
            max_recursion: 100,
        }
    }
}

impl SandboxConfig {
    /// Create a relaxed sandbox for development
    pub fn development() -> Self {
        Self {
            memory_limit: Some(10 * 1024 * 1024), // 10MB
            timeout_ms: 100,
            allow_filesystem: true, // Allow for dev convenience
            allow_network: false,
            allow_os: false,
            max_recursion: 1000,
        }
    }

    /// Create a strict sandbox for production
    pub fn production() -> Self {
        Self {
            memory_limit: Some(512 * 1024), // 512KB
            timeout_ms: 5,
            allow_filesystem: false,
            allow_network: false,
            allow_os: false,
            max_recursion: 50,
        }
    }
}

/// Runtime limits for script execution
#[derive(Debug, Clone, Copy, Default)]
pub struct SandboxLimits {
    /// Current memory usage in bytes
    pub memory_used: usize,

    /// Current execution time in microseconds
    pub execution_time_us: u64,

    /// Current recursion depth
    pub recursion_depth: u32,

    /// Number of instructions executed
    pub instruction_count: u64,
}

impl SandboxLimits {
    /// Check if any limit has been exceeded
    pub fn check(&self, config: &SandboxConfig) -> Result<(), SandboxViolation> {
        if let Some(memory_limit) = config.memory_limit {
            if self.memory_used > memory_limit {
                return Err(SandboxViolation::MemoryLimit {
                    used: self.memory_used,
                    limit: memory_limit,
                });
            }
        }

        if self.execution_time_us > config.timeout_ms * 1000 {
            return Err(SandboxViolation::Timeout {
                elapsed_ms: self.execution_time_us / 1000,
                limit_ms: config.timeout_ms,
            });
        }

        if self.recursion_depth > config.max_recursion {
            return Err(SandboxViolation::RecursionLimit {
                depth: self.recursion_depth,
                limit: config.max_recursion,
            });
        }

        Ok(())
    }
}

/// Sandbox violation types
#[derive(Debug, Clone)]
pub enum SandboxViolation {
    MemoryLimit { used: usize, limit: usize },
    Timeout { elapsed_ms: u64, limit_ms: u64 },
    RecursionLimit { depth: u32, limit: u32 },
    FileSystemAccess { path: String },
    NetworkAccess { address: String },
    OsCommand { command: String },
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxViolation::MemoryLimit { used, limit } => {
                write!(
                    f,
                    "Memory limit exceeded: {} bytes (limit: {})",
                    used, limit
                )
            }
            SandboxViolation::Timeout {
                elapsed_ms,
                limit_ms,
            } => {
                write!(
                    f,
                    "Script timeout: {}ms (limit: {}ms)",
                    elapsed_ms, limit_ms
                )
            }
            SandboxViolation::RecursionLimit { depth, limit } => {
                write!(f, "Recursion limit exceeded: {} (limit: {})", depth, limit)
            }
            SandboxViolation::FileSystemAccess { path } => {
                write!(f, "File system access denied: {}", path)
            }
            SandboxViolation::NetworkAccess { address } => {
                write!(f, "Network access denied: {}", address)
            }
            SandboxViolation::OsCommand { command } => {
                write!(f, "OS command execution denied: {}", command)
            }
        }
    }
}
