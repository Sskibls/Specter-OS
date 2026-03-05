//! Privileged execution controls for PhantomKernel OS
//!
//! This crate provides capability-based execution boundaries for privileged
//! operations such as nftables manipulation, route configuration, and other
//! system-level network operations.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    NetAdmin,
    NetRaw,
    SysAdmin,
    DacOverride,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::NetAdmin => "net_admin",
            Capability::NetRaw => "net_raw",
            Capability::SysAdmin => "sys_admin",
            Capability::DacOverride => "dac_override",
        }
    }
}

impl std::str::FromStr for Capability {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "net_admin" => Ok(Capability::NetAdmin),
            "net_raw" => Ok(Capability::NetRaw),
            "sys_admin" => Ok(Capability::SysAdmin),
            "dac_override" => Ok(Capability::DacOverride),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivExecError {
    CapabilityNotHeld(Capability),
    CommandNotAllowed(String),
    ExecutionFailed(String),
    ValidationError(String),
    IoError(String),
}

impl Display for PrivExecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivExecError::CapabilityNotHeld(cap) => {
                write!(formatter, "capability not held: {}", cap.as_str())
            }
            PrivExecError::CommandNotAllowed(cmd) => {
                write!(formatter, "command not allowed: {cmd}")
            }
            PrivExecError::ExecutionFailed(msg) => {
                write!(formatter, "execution failed: {msg}")
            }
            PrivExecError::ValidationError(msg) => {
                write!(formatter, "validation error: {msg}")
            }
            PrivExecError::IoError(msg) => {
                write!(formatter, "I/O error: {msg}")
            }
        }
    }
}

impl Error for PrivExecError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicy {
    pub program: String,
    pub allowed_args: Vec<String>,
    pub required_capabilities: Vec<Capability>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct PrivilegedExecutor {
    held_capabilities: HashSet<Capability>,
    command_policies: Vec<CommandPolicy>,
    execution_log: Vec<ExecutionRecord>,
    enforcing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub timestamp: u64,
    pub program: String,
    pub args: Vec<String>,
    pub success: bool,
    pub capabilities_used: Vec<Capability>,
}

impl PrivilegedExecutor {
    pub fn new(enforcing: bool) -> Self {
        let mut executor = Self {
            held_capabilities: HashSet::new(),
            command_policies: Vec::new(),
            execution_log: Vec::new(),
            enforcing,
        };

        executor.install_default_policies();
        executor
    }

    pub fn with_capabilities(capabilities: Vec<Capability>, enforcing: bool) -> Self {
        let mut executor = Self {
            held_capabilities: capabilities.into_iter().collect(),
            command_policies: Vec::new(),
            execution_log: Vec::new(),
            enforcing,
        };

        executor.install_default_policies();
        executor
    }

    fn install_default_policies(&mut self) {
        self.command_policies = vec![
            CommandPolicy {
                program: "nft".to_string(),
                allowed_args: vec![
                    "add".to_string(),
                    "rule".to_string(),
                    "flush".to_string(),
                    "chain".to_string(),
                    "inet".to_string(),
                    "phantomkernel".to_string(),
                    "output".to_string(),
                    "drop".to_string(),
                    "accept".to_string(),
                ],
                required_capabilities: vec![Capability::NetAdmin],
                description: "nftables rule manipulation".to_string(),
            },
            CommandPolicy {
                program: "ip".to_string(),
                allowed_args: vec![
                    "route".to_string(),
                    "replace".to_string(),
                    "default".to_string(),
                    "dev".to_string(),
                    "via".to_string(),
                    "add".to_string(),
                    "del".to_string(),
                ],
                required_capabilities: vec![Capability::NetAdmin],
                description: "IP route configuration".to_string(),
            },
            CommandPolicy {
                program: "iptables".to_string(),
                allowed_args: vec![
                    "-A".to_string(),
                    "-D".to_string(),
                    "-F".to_string(),
                    "-L".to_string(),
                    "-N".to_string(),
                    "-X".to_string(),
                ],
                required_capabilities: vec![Capability::NetAdmin],
                description: "iptables rule manipulation".to_string(),
            },
        ];
    }

    pub fn held_capabilities(&self) -> Vec<Capability> {
        self.held_capabilities.iter().copied().collect()
    }

    pub fn is_enforcing(&self) -> bool {
        self.enforcing
    }

    pub fn set_enforcing(&mut self, enforcing: bool) {
        self.enforcing = enforcing;
    }

    pub fn validate_command(
        &self,
        program: &str,
        args: &[String],
    ) -> Result<Vec<Capability>, PrivExecError> {
        let policy = self
            .command_policies
            .iter()
            .find(|p| p.program == program)
            .ok_or_else(|| {
                PrivExecError::CommandNotAllowed(format!("unknown program: {program}"))
            })?;

        for arg in args {
            if !policy.allowed_args.iter().any(|allowed| {
                arg.starts_with(allowed.as_str())
                    || arg == "-"
                    || arg.starts_with('-')
                    || arg.starts_with('/')
                    || arg.parse::<u32>().is_ok()
                    || ["eth0", "eth1", "tun0", "tun1", "lo", "127.0.0.1"].contains(&arg.as_str())
            }) {
                return Err(PrivExecError::ValidationError(format!(
                    "argument '{arg}' not allowed for {program}"
                )));
            }
        }

        let missing: Vec<Capability> = policy
            .required_capabilities
            .iter()
            .filter(|cap| !self.held_capabilities.contains(cap))
            .copied()
            .collect();

        if !missing.is_empty() {
            return Err(PrivExecError::CapabilityNotHeld(missing[0]));
        }

        Ok(policy.required_capabilities.clone())
    }

    pub fn run(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<String, PrivExecError> {
        let caps = self.validate_command(program, args)?;

        if !self.enforcing {
            let record = ExecutionRecord {
                timestamp: current_timestamp(),
                program: program.to_string(),
                args: args.to_vec(),
                success: true,
                capabilities_used: caps.clone(),
            };
            self.execution_log.push(record);
            return Ok("staged (non-enforcing)".to_string());
        }

        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| PrivExecError::ExecutionFailed(error.to_string()))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);

        let record = ExecutionRecord {
            timestamp: current_timestamp(),
            program: program.to_string(),
            args: args.to_vec(),
            success,
            capabilities_used: caps,
        };
        self.execution_log.push(record);

        if !success {
            return Err(PrivExecError::ExecutionFailed(stderr.to_string()));
        }

        Ok(stdout)
    }

    pub fn execution_log(&self) -> &[ExecutionRecord] {
        &self.execution_log
    }

    pub fn clear_log(&mut self) {
        self.execution_log.clear();
    }

    pub fn save_policy(&self, path: &Path) -> Result<(), PrivExecError> {
        let content = serde_json::to_string_pretty(&self.command_policies)
            .map_err(|error| PrivExecError::IoError(error.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| PrivExecError::IoError(error.to_string()))?;
        }

        let mut file = fs::File::create(path)
            .map_err(|error| PrivExecError::IoError(error.to_string()))?;
        file.write_all(content.as_bytes())
            .map_err(|error| PrivExecError::IoError(error.to_string()))?;

        Ok(())
    }

    pub fn load_policy(&mut self, path: &Path) -> Result<(), PrivExecError> {
        let content = fs::read_to_string(path)
            .map_err(|error| PrivExecError::IoError(error.to_string()))?;

        let policies: Vec<CommandPolicy> = serde_json::from_str(&content)
            .map_err(|error| PrivExecError::ValidationError(error.to_string()))?;

        self.command_policies = policies;
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub effective: HashSet<Capability>,
    pub permitted: HashSet<Capability>,
    pub inheritable: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn current() -> Self {
        Self {
            effective: HashSet::new(),
            permitted: HashSet::new(),
            inheritable: HashSet::new(),
        }
    }

    pub fn from_env(env_caps: &str) -> Self {
        let mut effective = HashSet::new();
        let mut permitted = HashSet::new();
        let mut inheritable = HashSet::new();

        for cap_str in env_caps.split(',') {
            let parts: Vec<&str> = cap_str.split('=').collect();
            if parts.len() == 2 {
                let set_type = parts[0];
                let caps = parts[1];

                for cap_name in caps.split('+') {
                    if let Ok(cap) = cap_name.parse::<Capability>() {
                        match set_type {
                            "e" => {
                                effective.insert(cap);
                            }
                            "p" => {
                                permitted.insert(cap);
                            }
                            "i" => {
                                inheritable.insert(cap);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Self {
            effective,
            permitted,
            inheritable,
        }
    }

    pub fn has_capability(&self, cap: Capability) -> bool {
        self.effective.contains(&cap)
    }

    pub fn to_env_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.effective.is_empty() {
            let caps: Vec<_> = self.effective.iter().map(|c| c.as_str()).collect();
            parts.push(format!("e={}", caps.join("+")));
        }

        if !self.permitted.is_empty() {
            let caps: Vec<_> = self.permitted.iter().map(|c| c.as_str()).collect();
            parts.push(format!("p={}", caps.join("+")));
        }

        if !self.inheritable.is_empty() {
            let caps: Vec<_> = self.inheritable.iter().map(|c| c.as_str()).collect();
            parts.push(format!("i={}", caps.join("+")));
        }

        parts.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_from_str_valid() {
        assert_eq!("net_admin".parse::<Capability>(), Ok(Capability::NetAdmin));
        assert_eq!("net_raw".parse::<Capability>(), Ok(Capability::NetRaw));
        assert_eq!("sys_admin".parse::<Capability>(), Ok(Capability::SysAdmin));
        assert_eq!(
            "dac_override".parse::<Capability>(),
            Ok(Capability::DacOverride)
        );
        assert_eq!("invalid".parse::<Capability>(), Err(()));
    }

    #[test]
    fn executor_validates_allowed_commands() {
        let mut executor = PrivilegedExecutor::new(false);
        executor.held_capabilities.insert(Capability::NetAdmin);

        let args = vec!["add".to_string(), "rule".to_string()];
        let result = executor.validate_command("nft", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn executor_rejects_unknown_program() {
        let executor = PrivilegedExecutor::new(false);
        let args = vec!["-c".to_string(), "echo test".to_string()];
        let result = executor.validate_command("unknown", &args);
        assert!(matches!(result, Err(PrivExecError::CommandNotAllowed(_))));
    }

    #[test]
    fn executor_requires_capabilities() {
        let executor = PrivilegedExecutor::new(false);
        let args = vec!["add".to_string(), "rule".to_string()];
        let result = executor.validate_command("nft", &args);
        assert!(matches!(result, Err(PrivExecError::CapabilityNotHeld(_))));
    }

    #[test]
    fn executor_staged_mode_does_not_execute() {
        let mut executor = PrivilegedExecutor::new(false);
        executor.held_capabilities.insert(Capability::NetAdmin);

        let args = vec!["add".to_string(), "rule".to_string()];
        let result = executor.run("nft", &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "staged (non-enforcing)");
    }

    #[test]
    fn capability_set_from_env() {
        let caps = CapabilitySet::from_env("e=net_admin+net_raw,p=sys_admin");
        assert!(caps.has_capability(Capability::NetAdmin));
        assert!(caps.has_capability(Capability::NetRaw));
        assert!(!caps.has_capability(Capability::SysAdmin));
    }

    #[test]
    fn capability_set_to_env_string() {
        let mut caps = CapabilitySet::current();
        caps.effective.insert(Capability::NetAdmin);
        caps.effective.insert(Capability::NetRaw);
        caps.permitted.insert(Capability::SysAdmin);

        let env_str = caps.to_env_string();
        // Check format: should have e=... and p=... sections
        assert!(env_str.contains("e="), "Should contain effective caps");
        assert!(env_str.contains("p="), "Should contain permitted caps");
        // Check individual capabilities are present (order not guaranteed)
        assert!(env_str.contains("net_admin") || env_str.contains("net_raw"), "Should have effective caps");
        assert!(env_str.contains("sys_admin"), "Should have sys_admin in permitted");
    }

    #[test]
    fn executor_saves_and_loads_policy() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("policy.json");

        let executor = PrivilegedExecutor::new(false);
        executor.save_policy(&path).expect("policy should save");

        let mut loaded = PrivilegedExecutor::new(false);
        loaded.load_policy(&path).expect("policy should load");

        assert_eq!(executor.command_policies.len(), loaded.command_policies.len());
    }
}
