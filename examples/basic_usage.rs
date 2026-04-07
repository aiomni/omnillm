//! Demonstrates basic canonical-hybrid usage of the omnillm crate.
//!
//! Run with:
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example basic_usage
//! ```

use std::sync::Arc;

use omnillm::{
    GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("set OPENAI_API_KEY");

    let gateway = Arc::new(
        GatewayBuilder::new(ProviderEndpoint::openai_responses())
            .add_key(
                KeyConfig::new(&api_key, "openai-prod-1")
                    .tpm_limit(90_000)
                    .rpm_limit(500),
            )
            .budget_limit_usd(50.0)
            .build()
            .expect("failed to build gateway"),
    );

    let mut handles = Vec::new();
    for i in 0..5 {
        let gw = Arc::clone(&gateway);
        handles.push(tokio::spawn(async move {
            let req = LlmRequest {
                model: "gpt-4.1-mini".into(),
                instructions: Some("Answer in one short sentence.".into()),
                input: vec![RequestItem::from(Message::text(
                    MessageRole::User,
                    format!("Say hello, this is request number {i}"),
                ))],
                messages: Vec::new(),
                capabilities: Default::default(),
                generation: GenerationConfig {
                    max_output_tokens: Some(64),
                    ..Default::default()
                },
                metadata: Default::default(),
                vendor_extensions: Default::default(),
            };
            match gw.call(req, CancellationToken::new()).await {
                Ok(resp) => println!(
                    "[{i}] ✓ {} tokens — {}",
                    resp.usage.total(),
                    resp.content_text
                ),
                Err(e) => println!("[{i}] ✗ {e}"),
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    println!("\n─── Pool Status ───");
    for s in gateway.pool_status() {
        println!(
            "  {:20} available={:<5} inflight={:>6}/{:<6} failures={}",
            s.label, s.available, s.tpm_inflight, s.tpm_limit, s.consecutive_failures,
        );
    }
    println!(
        "  Budget: ${:.4} used, ${:.4} remaining",
        gateway.budget_used_usd(),
        gateway.budget_remaining_usd(),
    );
}
