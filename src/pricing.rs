//! Model pricing and cost estimation.

use crate::budget::tracker::{usd_to_micro, MicroDollar};
use crate::types::{PromptCacheUsage, TokenUsage};

/// Pricing for a specific model, in micro-dollars per 1k tokens.
struct ModelPricing {
    /// Cost per 1k input tokens in micro-dollars.
    input_per_1k: MicroDollar,
    /// Cost per 1k cached OpenAI-style input tokens in micro-dollars.
    cached_input_per_1k: Option<MicroDollar>,
    /// Cost per 1k Claude-style cache hit/read tokens in micro-dollars.
    cache_read_per_1k: Option<MicroDollar>,
    /// Cost per 1k short-lived Claude-style cache write tokens in micro-dollars.
    cache_write_short_per_1k: Option<MicroDollar>,
    /// Cost per 1k long-lived Claude-style cache write tokens in micro-dollars.
    cache_write_long_per_1k: Option<MicroDollar>,
    /// Cost per 1k output tokens in micro-dollars.
    output_per_1k: MicroDollar,
}

impl ModelPricing {
    fn basic(input_per_1k: f64, output_per_1k: f64) -> Self {
        Self {
            input_per_1k: usd_to_micro(input_per_1k),
            cached_input_per_1k: None,
            cache_read_per_1k: None,
            cache_write_short_per_1k: None,
            cache_write_long_per_1k: None,
            output_per_1k: usd_to_micro(output_per_1k),
        }
    }

    fn openai(input_per_1k: f64, cached_input_per_1k: f64, output_per_1k: f64) -> Self {
        Self {
            input_per_1k: usd_to_micro(input_per_1k),
            cached_input_per_1k: Some(usd_to_micro(cached_input_per_1k)),
            cache_read_per_1k: None,
            cache_write_short_per_1k: None,
            cache_write_long_per_1k: None,
            output_per_1k: usd_to_micro(output_per_1k),
        }
    }

    fn claude(
        input_per_1k: f64,
        cache_write_short_per_1k: f64,
        cache_write_long_per_1k: f64,
        cache_read_per_1k: f64,
        output_per_1k: f64,
    ) -> Self {
        Self {
            input_per_1k: usd_to_micro(input_per_1k),
            cached_input_per_1k: None,
            cache_read_per_1k: Some(usd_to_micro(cache_read_per_1k)),
            cache_write_short_per_1k: Some(usd_to_micro(cache_write_short_per_1k)),
            cache_write_long_per_1k: Some(usd_to_micro(cache_write_long_per_1k)),
            output_per_1k: usd_to_micro(output_per_1k),
        }
    }
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
    let input = prompt_cost(usage, &p);
    let output = (usage.completion_tokens as u64 * p.output_per_1k) / 1000;
    input + output
}

fn prompt_cost(usage: &TokenUsage, pricing: &ModelPricing) -> MicroDollar {
    let Some(prompt_cache) = usage.prompt_cache.as_ref() else {
        return (usage.prompt_tokens as u64 * pricing.input_per_1k) / 1000;
    };

    openai_cached_prompt_cost(usage, prompt_cache, pricing)
        + claude_cache_extra_cost(prompt_cache, pricing)
}

fn openai_cached_prompt_cost(
    usage: &TokenUsage,
    prompt_cache: &PromptCacheUsage,
    pricing: &ModelPricing,
) -> MicroDollar {
    let Some(cached_rate) = pricing.cached_input_per_1k else {
        return (usage.prompt_tokens as u64 * pricing.input_per_1k) / 1000;
    };

    let cached_tokens = prompt_cache
        .cached_input_tokens
        .unwrap_or(0)
        .min(usage.prompt_tokens);
    let uncached_tokens = usage.prompt_tokens.saturating_sub(cached_tokens);

    ((uncached_tokens as u64 * pricing.input_per_1k) / 1000)
        + ((cached_tokens as u64 * cached_rate) / 1000)
}

fn claude_cache_extra_cost(prompt_cache: &PromptCacheUsage, pricing: &ModelPricing) -> MicroDollar {
    if pricing.cache_read_per_1k.is_none()
        && pricing.cache_write_short_per_1k.is_none()
        && pricing.cache_write_long_per_1k.is_none()
    {
        return 0;
    }

    let read = prompt_cache
        .cache_read_input_tokens
        .zip(pricing.cache_read_per_1k)
        .map(|(tokens, rate)| (tokens as u64 * rate) / 1000)
        .unwrap_or(0);

    let creation = if prompt_cache.cache_creation_short_input_tokens.is_some()
        || prompt_cache.cache_creation_long_input_tokens.is_some()
    {
        prompt_cache
            .cache_creation_short_input_tokens
            .zip(pricing.cache_write_short_per_1k)
            .map(|(tokens, rate)| (tokens as u64 * rate) / 1000)
            .unwrap_or(0)
            + prompt_cache
                .cache_creation_long_input_tokens
                .zip(pricing.cache_write_long_per_1k)
                .map(|(tokens, rate)| (tokens as u64 * rate) / 1000)
                .unwrap_or(0)
    } else {
        prompt_cache
            .cache_creation_input_tokens
            .zip(pricing.cache_write_short_per_1k)
            .map(|(tokens, rate)| (tokens as u64 * rate) / 1000)
            .unwrap_or(0)
    };

    read + creation
}

/// Look up pricing for a model. Unknown models are charged at GPT-4o rates
/// as a conservative default.
fn pricing(model: &str) -> ModelPricing {
    match model {
        m if m.starts_with("gpt-5.5") => ModelPricing::openai(0.005, 0.0005, 0.030),
        m if m.starts_with("gpt-5.4-mini") => ModelPricing::openai(0.000750, 0.000075, 0.004500),
        m if m.starts_with("gpt-5.4") => ModelPricing::openai(0.002500, 0.000250, 0.015000),
        m if m.starts_with("gpt-4o-mini") => ModelPricing::basic(0.000150, 0.000600),
        m if m.starts_with("gpt-4o") => ModelPricing::basic(0.005, 0.015),
        m if m.starts_with("claude-opus-4") => {
            ModelPricing::claude(0.005, 0.00625, 0.010, 0.00050, 0.025)
        }
        m if m.starts_with("claude-sonnet-4") || m.starts_with("claude-3-5-sonnet") => {
            ModelPricing::claude(0.003, 0.00375, 0.006, 0.00030, 0.015)
        }
        m if m.starts_with("claude-haiku-4") => {
            ModelPricing::claude(0.001, 0.00125, 0.002, 0.00010, 0.005)
        }
        m if m.starts_with("claude-3-haiku") => {
            ModelPricing::claude(0.00025, 0.00030, 0.00050, 0.00003, 0.00125)
        }
        _ => ModelPricing::basic(0.005, 0.015),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(
        prompt_tokens: u32,
        completion_tokens: u32,
        prompt_cache: PromptCacheUsage,
    ) -> TokenUsage {
        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: None,
            prompt_cache: Some(prompt_cache),
        }
    }

    #[test]
    fn estimate_does_not_assume_prompt_cache_discount() {
        assert_eq!(estimate(1_000, "gpt-5.4"), usd_to_micro(0.0025));
    }

    #[test]
    fn actual_uses_openai_cached_input_rate_when_available() {
        let usage = usage(
            1_000,
            100,
            PromptCacheUsage {
                cached_input_tokens: Some(400),
                ..Default::default()
            },
        );

        assert_eq!(actual(&usage, "gpt-5.4"), 3_100);
    }

    #[test]
    fn actual_uses_claude_cache_read_and_generic_write_rates() {
        let usage = usage(
            100,
            50,
            PromptCacheUsage {
                cache_read_input_tokens: Some(1_000),
                cache_creation_input_tokens: Some(2_000),
                ..Default::default()
            },
        );

        assert_eq!(actual(&usage, "claude-3-5-sonnet-20241022"), 8_850);
    }

    #[test]
    fn actual_uses_claude_specific_short_and_long_write_rates() {
        let usage = usage(
            0,
            0,
            PromptCacheUsage {
                cache_creation_short_input_tokens: Some(1_000),
                cache_creation_long_input_tokens: Some(1_000),
                ..Default::default()
            },
        );

        assert_eq!(actual(&usage, "claude-sonnet-4-5"), 9_750);
    }

    #[test]
    fn unknown_cache_rates_fall_back_to_uncached_prompt_cost() {
        let usage = usage(
            1_000,
            0,
            PromptCacheUsage {
                cached_input_tokens: Some(900),
                cache_read_input_tokens: Some(2_000),
                cache_creation_input_tokens: Some(2_000),
                ..Default::default()
            },
        );

        assert_eq!(actual(&usage, "unknown-model"), usd_to_micro(0.005));
    }
}
