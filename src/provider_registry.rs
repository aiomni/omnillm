//! Embedded provider and endpoint support metadata.

use serde::{Deserialize, Serialize};

use crate::api::{EndpointKind, ProviderKind, WireFormat};

/// Support status for a provider endpoint family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    Native,
    Compatible,
    Planned,
}

/// Supported endpoint family for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSupport {
    pub endpoint: EndpointKind,
    pub level: SupportLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wire_formats: Vec<WireFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl EndpointSupport {
    pub fn supports_wire_format(&self, wire_format: WireFormat) -> bool {
        self.wire_formats.contains(&wire_format)
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self.level, SupportLevel::Planned) && !self.wire_formats.is_empty()
    }
}

/// Provider-level support metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub kind: ProviderKind,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<EndpointSupport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl ProviderDescriptor {
    pub fn endpoint(&self, endpoint: EndpointKind) -> Option<&EndpointSupport> {
        self.endpoints.iter().find(|item| item.endpoint == endpoint)
    }

    pub fn supports_endpoint(&self, endpoint: EndpointKind) -> bool {
        self.endpoint(endpoint)
            .map(EndpointSupport::is_enabled)
            .unwrap_or(false)
    }

    pub fn supports_wire_format(&self, wire_format: WireFormat) -> bool {
        self.endpoints
            .iter()
            .any(|endpoint| endpoint.supports_wire_format(wire_format))
    }
}

/// Embedded provider registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRegistry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ProviderDescriptor>,
}

impl ProviderRegistry {
    pub fn embedded() -> Self {
        serde_json::from_str(include_str!("../support/provider_support_matrix.json"))
            .expect("embedded provider support matrix should be valid")
    }

    pub fn provider(&self, kind: ProviderKind) -> Option<&ProviderDescriptor> {
        self.providers.iter().find(|provider| provider.kind == kind)
    }

    pub fn supports_endpoint(&self, provider: ProviderKind, endpoint: EndpointKind) -> bool {
        self.provider(provider)
            .map(|item| item.supports_endpoint(endpoint))
            .unwrap_or(false)
    }

    pub fn supports_wire_format(&self, provider: ProviderKind, wire_format: WireFormat) -> bool {
        self.provider(provider)
            .map(|item| item.supports_wire_format(wire_format))
            .unwrap_or(false)
    }
}

pub fn embedded_provider_registry() -> ProviderRegistry {
    ProviderRegistry::embedded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_contains_expected_provider_entries() {
        let registry = ProviderRegistry::embedded();

        let openai = registry
            .provider(ProviderKind::OpenAi)
            .expect("openai provider should be present");
        assert!(openai.supports_wire_format(WireFormat::OpenAiResponses));
        assert!(openai.supports_wire_format(WireFormat::OpenAiEmbeddings));

        let bedrock = registry
            .provider(ProviderKind::Bedrock)
            .expect("bedrock provider should be present");
        let messages = bedrock
            .endpoint(EndpointKind::Messages)
            .expect("bedrock should expose a messages entry");
        assert_eq!(messages.level, SupportLevel::Planned);
    }
}
