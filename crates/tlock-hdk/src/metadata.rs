//! Plugin metadata for security, privacy, and decentralization aspects.
//!
//! Inspired by Nix's `unfree` channel, this module provides a standard way
//! for plugins to declare their security posture so users can make informed
//! decisions before installing or running a plugin.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Security, privacy, and decentralization metadata for a plugin.
///
/// Each field surfaces an attribute that helps users understand the
/// trust and decentralization profile of a plugin and the on-chain or
/// external resources it interacts with.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PluginMetadata {
    /// Whether the plugin's source code is publicly available.
    pub open_source: bool,

    /// Whether the smart contracts the plugin interacts with have been
    /// independently audited.
    pub audited_contract: bool,

    /// Whether the plugin or its authors have safe-harbor protection.
    pub safe_harbor: bool,

    /// Whether the plugin has an active bug-bounty programme.
    pub bug_bounty: bool,

    /// Whether the plugin relies on centralized infrastructure
    /// (e.g. Cloudflare, AWS, Infura, Alchemy).
    pub centralized_infrastructure: bool,

    /// Names of centralized services the plugin depends on.
    pub centralized_services: Vec<String>,

    /// Whether the plugin collects, reports, or logs user data.
    pub reports_data: bool,

    /// Human-readable description of what data is collected and why.
    pub data_reporting_description: Option<String>,

    /// URL to the source-code repository.
    pub source_url: Option<String>,

    /// URL to the audit report(s).
    pub audit_url: Option<String>,

    /// URL to the bug-bounty programme.
    pub bug_bounty_url: Option<String>,

    /// SPDX licence identifier or free-text licence name.
    pub license: Option<String>,

    /// Plugin version string (semver recommended).
    pub version: Option<String>,

    /// Short human-readable description of the plugin.
    pub description: Option<String>,
}

impl PluginMetadata {
    /// Create an empty metadata builder.
    pub fn new() -> Self {
        Self::default()
    }

    // --- Builder methods ------------------------------------------------

    pub fn open_source(mut self, v: bool) -> Self {
        self.open_source = v;
        self
    }

    pub fn audited_contract(mut self, v: bool) -> Self {
        self.audited_contract = v;
        self
    }

    pub fn safe_harbor(mut self, v: bool) -> Self {
        self.safe_harbor = v;
        self
    }

    pub fn bug_bounty(mut self, v: bool) -> Self {
        self.bug_bounty = v;
        self
    }

    pub fn centralized_infrastructure(mut self, v: bool) -> Self {
        self.centralized_infrastructure = v;
        self
    }

    /// Mark a centralized service dependency.  Automatically sets
    /// `centralized_infrastructure` to `true`.
    pub fn centralized_service(mut self, name: impl Into<String>) -> Self {
        self.centralized_services.push(name.into());
        self.centralized_infrastructure = true;
        self
    }

    pub fn reports_data(mut self, v: bool) -> Self {
        self.reports_data = v;
        self
    }

    pub fn data_reporting_description(mut self, desc: impl Into<String>) -> Self {
        self.data_reporting_description = Some(desc.into());
        self
    }

    pub fn source_url(mut self, url: impl Into<String>) -> Self {
        self.source_url = Some(url.into());
        self
    }

    pub fn audit_url(mut self, url: impl Into<String>) -> Self {
        self.audit_url = Some(url.into());
        self
    }

    pub fn bug_bounty_url(mut self, url: impl Into<String>) -> Self {
        self.bug_bounty_url = Some(url.into());
        self
    }

    pub fn license(mut self, lic: impl Into<String>) -> Self {
        self.license = Some(lic.into());
        self
    }

    pub fn version(mut self, ver: impl Into<String>) -> Self {
        self.version = Some(ver.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Returns `true` if the plugin is fully transparent: open-source
    /// with audited contracts and no centralized dependencies or data
    /// reporting.
    pub fn is_fully_decentralized(&self) -> bool {
        self.open_source
            && self.audited_contract
            && !self.centralized_infrastructure
            && !self.reports_data
    }
}

impl fmt::Display for PluginMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Plugin Metadata:")?;
        if let Some(ref desc) = self.description {
            writeln!(f, "  Description: {desc}")?;
        }
        if let Some(ref ver) = self.version {
            writeln!(f, "  Version: {ver}")?;
        }
        if let Some(ref lic) = self.license {
            writeln!(f, "  License: {lic}")?;
        }
        writeln!(f, "  Open Source: {}", yes_no(self.open_source))?;
        writeln!(f, "  Audited Contract: {}", yes_no(self.audited_contract))?;
        writeln!(f, "  Safe Harbor: {}", yes_no(self.safe_harbor))?;
        writeln!(f, "  Bug Bounty: {}", yes_no(self.bug_bounty))?;
        writeln!(
            f,
            "  Centralized Infrastructure: {}",
            yes_no(self.centralized_infrastructure)
        )?;
        if !self.centralized_services.is_empty() {
            writeln!(
                f,
                "  Centralized Services: {}",
                self.centralized_services.join(", ")
            )?;
        }
        writeln!(f, "  Reports Data: {}", yes_no(self.reports_data))?;
        if let Some(ref desc) = self.data_reporting_description {
            writeln!(f, "  Data Reporting: {desc}")?;
        }
        if let Some(ref url) = self.source_url {
            writeln!(f, "  Source URL: {url}")?;
        }
        if let Some(ref url) = self.audit_url {
            writeln!(f, "  Audit URL: {url}")?;
        }
        if let Some(ref url) = self.bug_bounty_url {
            writeln!(f, "  Bug Bounty URL: {url}")?;
        }
        Ok(())
    }
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

/// Trait that every plugin should implement to expose its security,
/// privacy, and decentralization metadata.
///
/// Implement this on your plugin's main struct (or a dedicated marker
/// type) and return a fully populated `PluginMetadata`.
pub trait MetadataProvider {
    /// Return the plugin's metadata.
    fn metadata(&self) -> PluginMetadata;
}
