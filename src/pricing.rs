//! Model pricing and cost estimation.

use crate::budget::tracker::{usd_to_micro, MicroDollar};
use crate::types::TokenUsage;

/// Pricing for a specific model, in micro-dollars per 1k tokens.
struct ModelPricing {
    /// Cost per 1k input tokens in micro-dollars.
    input_per_1k: MicroDollar,
    /// Cost per 1k output tokens in micro-dollars.
    output_per_1k: MicroDollar,
}

/// Estimate cost from token count before the request is sent.
///
/// Uses total tokens with input pricing as a conservative upper bound.
pub(crate) fn estimate(tokens: u32, model: &str) -> MicroDollar {
    let p = pricing(model);
    (tokens as u64 * p.input_per_1k) / 1000
}

/// Compute actual cost from the response's usage report.
pub(crate) fn actual(usage: &TokenUsage, model: &str) -> MicroDollar {
    let p = pricing(model);
    let input = (usage.prompt_tokens as u64 * p.input_per_1k) / 1000;
    let output = (usage.completion_tokens as u64 * p.output_per_1k) / 1000;
    input + output
}

/// Look up pricing for a model. Unknown models are charged at GPT-4o rates
/// as a conservative default.
fn pricing(model: &str) -> ModelPricing {
    match model {
        m if m.starts_with("gpt-4o-mini") => ModelPricing {
            input_per_1k: usd_to_micro(0.000150),
            output_per_1k: usd_to_micro(0.000600),
        },
        m if m.starts_with("gpt-4o") => ModelPricing {
            input_per_1k: usd_to_micro(0.005),
            output_per_1k: usd_to_micro(0.015),
        },
        m if m.starts_with("claude-3-5-sonnet") => ModelPricing {
            input_per_1k: usd_to_micro(0.003),
            output_per_1k: usd_to_micro(0.015),
        },
        m if m.starts_with("claude-3-haiku") => ModelPricing {
            input_per_1k: usd_to_micro(0.00025),
            output_per_1k: usd_to_micro(0.00125),
        },
        _ => ModelPricing {
            // Unknown model: charge at GPT-4o rate (conservative).
            input_per_1k: usd_to_micro(0.005),
            output_per_1k: usd_to_micro(0.015),
        },
    }
}
