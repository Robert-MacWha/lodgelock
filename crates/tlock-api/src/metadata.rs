//! Security / privacy / decentralization metadata for plugins (extensions).
//!
//! Modeled after Nix's `unfree` channel: every plugin self-declares a set of
//! trust-relevant attributes so hosts and users can filter, warn about, or
//! refuse to load plugins that don't meet their policy. The metadata covers
//! both the plugin itself and the on-chain / external resources it interacts
//! with (audited contracts, centralized infrastructure, telemetry, ...).
//!
//! Defaults are deliberately *pessimistic*: anything a plugin does not declare
//! is assumed to be the riskier option, mirroring how Nix treats `unfree`
//! packages as opt-in.

use serde::{Deserialize, Serialize};

/// Complete metadata block describing a plugin's trust-relevant properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginMetadata {
    /// Plugin display name.
    pub name: String,
    /// Plugin semantic version.
    pub version: String,
    /// Short human-readable description.
    pub description: String,
    /// Source openness & licensing.
    pub openness: Openness,
    /// Security posture of the plugin and the contracts it interacts with.
    pub security: Security,
    /// Privacy / data-handling posture.
    pub privacy: Privacy,
    /// Decentralization posture.
    pub decentralization: Decentralization,
}

impl Default for PluginMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: String::new(),
            openness: Openness::default(),
            security: Security::default(),
            privacy: Privacy::default(),
            decentralization: Decentralization::default(),
        }
    }
}

/// Source openness & licensing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Openness {
    /// `true` if the plugin source is publicly available under an
    /// OSI-approved license.
    pub open_source: bool,
    /// SPDX license identifier when open source (e.g. `"MIT"`, `"Apache-2.0"`).
    pub license: Option<String>,
    /// URL of the public source repository, if any.
    pub repository_url: Option<String>,
}

impl Default for Openness {
    fn default() -> Self {
        Self { open_source: false, license: None, repository_url: None }
    }
}

/// Security posture of the plugin and the on-chain resources it touches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Security {
    /// `true` if the on-chain contracts the plugin interacts with have been
    /// professionally audited.
    pub audited_contract: bool,
    /// URLs of audit reports, if any.
    pub audit_reports: Vec<String>,
    /// `true` if the project maintains a safe-harbor policy for good-faith
    /// security researchers.
    pub safe_harbor: bool,
    /// `true` if there is an active bug-bounty program.
    pub bug_bounty: bool,
    /// URL of the bug-bounty program, if any.
    pub bug_bounty_url: Option<String>,
}

impl Default for Security {
    fn default() -> Self {
        Self {
            audited_contract: false,
            audit_reports: Vec::new(),
            safe_harbor: false,
            bug_bounty: false,
            bug_bounty_url: None,
        }
    }
}

/// Privacy / data-handling posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Privacy {
    /// `true` if the plugin reports telemetry or performs off-chain logging.
    /// Defaults to `true` (assume it logs until declared otherwise).
    pub reports_data: bool,
    /// Categories of data collected, if any.
    pub data_collected: Vec<DataCategory>,
    /// URL of the privacy policy, if any.
    pub privacy_policy_url: Option<String>,
}

impl Default for Privacy {
    fn default() -> Self {
        // Pessimistic: assume data is reported until the plugin opts out.
        Self {
            reports_data: true,
            data_collected: Vec::new(),
            privacy_policy_url: None,
        }
    }
}

/// A category of data a plugin may collect or report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    UsageAnalytics,
    WalletAddresses,
    TransactionHistory,
    IpAddress,
    DeviceFingerprint,
    PersonalIdentifiableInfo,
}

/// Decentralization posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Decentralization {
    /// `true` if the plugin relies on centralized infrastructure
    /// (Cloudflare, AWS, ...). Defaults to `true` (assume centralized until
    /// declared otherwise).
    pub relies_on_centralized_infrastructure: bool,
    /// Names of the centralized providers relied upon (e.g. `"aws"`,
    /// `"cloudflare"`).
    pub centralized_providers: Vec<String>,
    /// `true` if a single operator can censor or halt the plugin's critical
    /// path.
    pub single_operator_risk: bool,
}

impl Default for Decentralization {
    fn default() -> Self {
        Self {
            relies_on_centralized_infrastructure: true,
            centralized_providers: Vec::new(),
            single_operator_risk: true,
        }
    }
}

/// Coarse Nix-style "channel" classification derived from the metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustChannel {
    /// Meets every positive criterion: open source, audited, no centralized
    /// reliance, no data reporting.
    Trusted,
    /// Open source but with some remaining concerns.
    Restricted,
    /// Fails key criteria; analogous to Nix's `unfree`.
    Untrusted,
}

impl PluginMetadata {
    /// Parse a metadata manifest from a JSON string.
    pub fn from_json(manifest: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(manifest)
    }

    /// Serialize the metadata to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// `true` if the plugin carries any trust-relevant restriction, i.e. it is
    /// not fully open/audited/decentralized/private. Analogous to Nix's
    /// `unfree` predicate.
    pub fn has_restrictions(&self) -> bool {
        !self.openness.open_source
            || !self.security.audited_contract
            || self.privacy.reports_data
            || self.decentralization.relies_on_centralized_infrastructure
    }

    /// Compute the coarse trust channel for this plugin.
    pub fn trust_channel(&self) -> TrustChannel {
        let decentralized = !self.decentralization.relies_on_centralized_infrastructure
            && !self.decentralization.single_operator_risk;

        if self.openness.open_source
            && self.security.audited_contract
            && !self.privacy.reports_data
            && decentralized
        {
            TrustChannel::Trusted
        } else if self.openness.open_source {
            TrustChannel::Restricted
        } else {
            TrustChannel::Untrusted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_pessimistic() {
        let m = PluginMetadata::default();
        assert!(!m.openness.open_source);
        assert!(!m.security.audited_contract);
        assert!(m.privacy.reports_data);
        assert!(m.decentralization.relies_on_centralized_infrastructure);
        assert!(m.has_restrictions());
        assert_eq!(m.trust_channel(), TrustChannel::Untrusted);
    }

    #[test]
    fn parses_minimal_manifest_with_pessimistic_defaults() {
        let json = r#"{"name":"my-plugin","version":"0.1.0"}"#;
        let m = PluginMetadata::from_json(json).unwrap();
        assert_eq!(m.name, "my-plugin");
        assert!(m.privacy.reports_data);
        assert!(!m.openness.open_source);
    }

    #[test]
    fn trusted_channel_requires_all_criteria() {
        let json = r#"{
            "name": "vault",
            "version": "1.0.0",
            "openness": { "open_source": true, "license": "MIT" },
            "security": { "audited_contract": true, "bug_bounty": true },
            "privacy": { "reports_data": false },
            "decentralization": {
                "relies_on_centralized_infrastructure": false,
                "single_operator_risk": false
            }
        }"#;
        let m = PluginMetadata::from_json(json).unwrap();
        assert!(!m.has_restrictions());
        assert_eq!(m.trust_channel(), TrustChannel::Trusted);
    }

    #[test]
    fn open_but_unaudited_is_restricted() {
        let json = r#"{
            "name": "swap",
            "version": "2.0.0",
            "openness": { "open_source": true },
            "privacy": { "reports_data": false },
            "decentralization": {
                "relies_on_centralized_infrastructure": false,
                "single_operator_risk": false
            }
        }"#;
        let m = PluginMetadata::from_json(json).unwrap();
        assert_eq!(m.trust_channel(), TrustChannel::Restricted);
    }

    #[test]
    fn json_roundtrip() {
        let m = PluginMetadata::default();
        let s = m.to_json().unwrap();
        let back = PluginMetadata::from_json(&s).unwrap();
        assert_eq!(m, back);
    }
}
