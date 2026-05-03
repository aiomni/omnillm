use serde_json::Value;

use super::{
    CacheBreakpoint, CapabilitySet, GenerationConfig, LlmRequest, Message, MessagePart,
    MessageRole, PromptCacheKey, PromptCachePolicy, PromptCacheRetention, RequestItem,
    ToolDefinition, VendorExtensions,
};

/// Helper for constructing cache-friendly prompt layouts.
#[derive(Debug, Clone)]
pub struct PromptLayoutBuilder {
    model: String,
    instructions: Option<String>,
    tools: Vec<ToolDefinition>,
    stable_messages: Vec<Message>,
    dynamic_messages: Vec<Message>,
    generation: GenerationConfig,
    prompt_cache: Option<PromptCachePolicy>,
    generated_key: Option<GeneratedPromptCacheKey>,
}

#[derive(Debug, Clone)]
struct GeneratedPromptCacheKey {
    namespace: String,
    tenant_scope: Option<String>,
    required: bool,
    retention: PromptCacheRetention,
}

impl PromptLayoutBuilder {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            instructions: None,
            tools: Vec::new(),
            stable_messages: Vec::new(),
            dynamic_messages: Vec::new(),
            generation: GenerationConfig::default(),
            prompt_cache: None,
            generated_key: None,
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn stable_message(mut self, message: Message) -> Self {
        self.stable_messages.push(message);
        self
    }

    pub fn dynamic_message(mut self, message: Message) -> Self {
        self.dynamic_messages.push(message);
        self
    }

    pub fn user_input(self, text: impl Into<String>) -> Self {
        self.dynamic_message(Message::text(MessageRole::User, text))
    }

    pub fn dynamic_rag_context(self, value: Value) -> Self {
        self.dynamic_message(Message {
            role: MessageRole::User,
            parts: vec![MessagePart::Json { value }],
            raw_message: None,
            vendor_extensions: VendorExtensions::new(),
        })
    }

    pub fn generation(mut self, generation: GenerationConfig) -> Self {
        self.generation = generation;
        self
    }

    pub fn prompt_cache(mut self, policy: PromptCachePolicy) -> Self {
        self.prompt_cache = Some(policy);
        self.generated_key = None;
        self
    }

    pub fn stable_prefix_cache_key(
        mut self,
        namespace: impl Into<String>,
        tenant_scope: Option<impl Into<String>>,
        retention: PromptCacheRetention,
        required: bool,
    ) -> Self {
        self.generated_key = Some(GeneratedPromptCacheKey {
            namespace: namespace.into(),
            tenant_scope: tenant_scope.map(Into::into),
            required,
            retention,
        });
        self.prompt_cache = None;
        self
    }

    pub fn build(mut self) -> LlmRequest {
        if let Some(generated) = self.generated_key.take() {
            let key = PromptCacheKey::Explicit {
                value: self.generated_cache_key(&generated),
            };
            self.prompt_cache = Some(if generated.required {
                PromptCachePolicy::Required {
                    key: Some(key),
                    retention: generated.retention,
                    breakpoint: CacheBreakpoint::Auto,
                    vendor_extensions: VendorExtensions::new(),
                }
            } else {
                PromptCachePolicy::BestEffort {
                    key: Some(key),
                    retention: generated.retention,
                    breakpoint: CacheBreakpoint::Auto,
                    vendor_extensions: VendorExtensions::new(),
                }
            });
        }

        let mut messages = self.stable_messages;
        messages.extend(self.dynamic_messages);
        let input = messages.iter().cloned().map(RequestItem::from).collect();

        LlmRequest {
            model: self.model,
            instructions: self.instructions,
            input,
            messages,
            capabilities: CapabilitySet {
                tools: self.tools,
                prompt_cache: self.prompt_cache,
                ..Default::default()
            },
            generation: self.generation,
            metadata: VendorExtensions::new(),
            vendor_extensions: VendorExtensions::new(),
        }
    }

    fn generated_cache_key(&self, generated: &GeneratedPromptCacheKey) -> String {
        let fingerprint = serde_json::json!({
            "model": self.model,
            "instructions": self.instructions,
            "tools": self.tools,
            "stable_messages": self.stable_messages,
        });
        let hash = fnv1a64(fingerprint.to_string().as_bytes());
        match &generated.tenant_scope {
            Some(scope) => format!("{}:{}:{hash:016x}", generated.namespace, scope),
            None => format!("{}:{hash:016x}", generated.namespace),
        }
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
