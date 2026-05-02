//! Gateway — the main entry point for provider-neutral LLM API requests.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::budget::tracker::BudgetTracker;
use crate::config::{GatewayConfig, KeyConfig, PoolConfig};
use crate::dispatcher::Dispatcher;
use crate::error::{ApiError, GatewayError};
use crate::key::inner::KeyInner;
use crate::key::pool::{KeyPool, KeyStatus};
use crate::pricing;
use crate::primitive::{
    PrimitiveBudgetClass, PrimitiveProviderEndpoint, PrimitiveRealtimeSession, PrimitiveRequest,
    PrimitiveResponse, PrimitiveStreamEvent, PrimitiveStreamMode,
};
use crate::protocol::ProviderEndpoint;
use crate::types::{LlmRequest, LlmResponse, LlmStreamEvent, Message, MessageRole};

pub type GatewayStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, GatewayError>> + Send>>;
pub type PrimitiveGatewayStream =
    Pin<Box<dyn Stream<Item = Result<PrimitiveStreamEvent, GatewayError>> + Send>>;

/// The main LLM API gateway.
pub struct Gateway {
    pool: Arc<KeyPool>,
    budget: Arc<BudgetTracker>,
    dispatcher: Arc<Dispatcher>,
}

impl Gateway {
    pub async fn call(
        &self,
        req: LlmRequest,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, GatewayError> {
        let est_tokens = req.estimated_tokens();
        let est_cost = pricing::estimate(est_tokens, &req.model);

        let lease = self
            .pool
            .acquire(est_tokens)
            .ok_or(GatewayError::NoAvailableKey)?;

        if !self.budget.try_reserve(est_cost) {
            return Err(GatewayError::BudgetExceeded);
        }

        if !lease.inner.rpm_window.try_acquire() {
            self.budget.settle(est_cost, 0);
            return Err(GatewayError::RateLimited);
        }

        let result = tokio::select! {
            res = self.dispatcher.call(&lease, &req) => res,
            _ = cancel.cancelled() => Err(ApiError::Cancelled),
        };

        match &result {
            Ok(resp) => {
                let actual = pricing::actual(&resp.usage, &req.model);
                self.budget.settle(est_cost, actual);
                self.pool.report_success(&lease);
            }
            Err(ApiError::Cancelled) => {
                self.budget.settle(est_cost, 0);
            }
            Err(ApiError::RateLimited { .. })
            | Err(ApiError::Unauthorized)
            | Err(ApiError::Provider(_))
            | Err(ApiError::PrimitiveProvider(_)) => {
                self.budget.settle(est_cost, 0);
                self.pool
                    .report_error(&lease, result.as_ref().err().expect("checked above"));
            }
            Err(ApiError::Protocol(_)) => {
                self.budget.settle(est_cost, 0);
            }
        }

        result.map_err(map_api_error)
    }

    pub async fn stream(
        &self,
        req: LlmRequest,
        cancel: CancellationToken,
    ) -> Result<GatewayStream, GatewayError> {
        let est_tokens = req.estimated_tokens();
        let est_prompt_tokens = req.estimated_prompt_tokens();
        let est_cost = pricing::estimate(est_tokens, &req.model);

        let lease = self
            .pool
            .acquire(est_tokens)
            .ok_or(GatewayError::NoAvailableKey)?;

        if !self.budget.try_reserve(est_cost) {
            return Err(GatewayError::BudgetExceeded);
        }

        if !lease.inner.rpm_window.try_acquire() {
            self.budget.settle(est_cost, 0);
            return Err(GatewayError::RateLimited);
        }

        let inner = match self.dispatcher.stream(&lease, &req).await {
            Ok(stream) => stream,
            Err(err) => {
                self.budget.settle(est_cost, 0);
                if matches!(
                    err,
                    ApiError::Unauthorized
                        | ApiError::RateLimited { .. }
                        | ApiError::Provider(_)
                        | ApiError::PrimitiveProvider(_)
                ) {
                    self.pool.report_error(&lease, &err);
                }
                return Err(map_api_error(err));
            }
        };

        let budget = Arc::clone(&self.budget);
        let pool = Arc::clone(&self.pool);
        let model = req.model.clone();
        let provider_protocol = self.dispatcher.protocol();
        let stream = try_stream! {
            let mut inner = inner;
            let mut usage = None;
            let mut content = String::new();
            let mut generated_chars = 0;
            let mut completed = false;
            let mut seen_tools = std::collections::HashSet::new();

            loop {
                let next = tokio::select! {
                    _ = cancel.cancelled() => {
                        Some(Err(ApiError::Cancelled))
                    }
                    item = inner.next() => item,
                };

                match next {
                    Some(Ok(event)) => {
                        match &event {
                            LlmStreamEvent::ResponseStarted { .. } => {}
                            LlmStreamEvent::TextDelta { delta } => {
                                content.push_str(delta);
                                generated_chars += delta.len();
                            }
                            LlmStreamEvent::ToolCallDelta { call_id, name, delta } => {
                                generated_chars += delta.len();
                                if seen_tools.insert(call_id.clone()) {
                                    generated_chars += name.len();
                                }
                            }
                            LlmStreamEvent::ReasoningDelta { delta } => {
                                generated_chars += delta.len();
                            }
                            LlmStreamEvent::Usage { usage: event_usage } => {
                                usage = Some(event_usage.clone());
                            }
                            LlmStreamEvent::Completed { response } => {
                                usage = Some(response.usage.clone());
                                completed = true;
                                let actual = pricing::actual(&response.usage, &req.model);
                                budget.settle(est_cost, actual);
                                pool.report_success(&lease);
                            }
                            _ => {}
                        }
                        yield event;
                    }
                    Some(Err(err)) => {
                        let partial_tokens = (generated_chars / 4) as u32;
                        let actual = if let Some(u) = &usage {
                            pricing::actual(u, &req.model)
                        } else {
                            let partial_usage = crate::types::TokenUsage {
                                prompt_tokens: est_prompt_tokens,
                                completion_tokens: partial_tokens,
                                total_tokens: Some(est_prompt_tokens + partial_tokens),
                                prompt_cache: None,
                            };
                            pricing::actual(&partial_usage, &req.model)
                        };
                        budget.settle(est_cost, actual);
                        if matches!(
                            err,
                            ApiError::Unauthorized | ApiError::RateLimited { .. } | ApiError::Provider(_) | ApiError::PrimitiveProvider(_)
                        ) {
                            pool.report_error(&lease, &err);
                        }
                        Err(map_api_error(err))?;
                    }
                    None => {
                        if !completed {
                            let partial_tokens = (generated_chars / 4) as u32;
                            let final_usage = usage.clone().unwrap_or_else(|| crate::types::TokenUsage {
                                prompt_tokens: est_prompt_tokens,
                                completion_tokens: partial_tokens,
                                total_tokens: Some(est_prompt_tokens + partial_tokens),
                                prompt_cache: None,
                            });
                            let actual = pricing::actual(&final_usage, &req.model);
                            budget.settle(est_cost, actual);
                            pool.report_success(&lease);
                            let response = LlmResponse::from_message(
                                provider_protocol,
                                model.clone(),
                                Message::text(MessageRole::Assistant, content.clone()),
                                final_usage,
                            );
                            yield LlmStreamEvent::Completed { response };
                        }
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    pub async fn primitive_call(
        &self,
        req: PrimitiveRequest,
        cancel: CancellationToken,
    ) -> Result<PrimitiveResponse, GatewayError> {
        self.ensure_primitive_supported(&req)?;
        let est_tokens = req.estimated_tokens();
        let est_cost = primitive_estimated_cost(&req);

        let lease = self
            .pool
            .acquire(est_tokens)
            .ok_or(GatewayError::NoAvailableKey)?;

        if !self.budget.try_reserve(est_cost) {
            return Err(GatewayError::BudgetExceeded);
        }

        if !lease.inner.rpm_window.try_acquire() {
            self.budget.settle(est_cost, 0);
            return Err(GatewayError::RateLimited);
        }

        let result = tokio::select! {
            res = self.dispatcher.primitive_call(&lease, &req) => res,
            _ = cancel.cancelled() => Err(ApiError::Cancelled),
        };

        match &result {
            Ok(response) => {
                let actual = response
                    .usage
                    .as_ref()
                    .and_then(|usage| usage.token_usage.as_ref())
                    .map(|usage| pricing::actual(usage, req.model_name()))
                    .unwrap_or(est_cost);
                self.budget.settle(est_cost, actual);
                self.pool.report_success(&lease);
            }
            Err(ApiError::Cancelled) => {
                self.budget.settle(est_cost, 0);
            }
            Err(ApiError::RateLimited { .. })
            | Err(ApiError::Unauthorized)
            | Err(ApiError::Provider(_))
            | Err(ApiError::PrimitiveProvider(_)) => {
                self.budget.settle(est_cost, 0);
                self.pool
                    .report_error(&lease, result.as_ref().err().expect("checked above"));
            }
            Err(ApiError::Protocol(_)) => {
                self.budget.settle(est_cost, 0);
            }
        }

        result.map_err(map_api_error)
    }

    pub async fn primitive_stream(
        &self,
        req: PrimitiveRequest,
        cancel: CancellationToken,
    ) -> Result<PrimitiveGatewayStream, GatewayError> {
        self.ensure_primitive_supported(&req)?;
        if req.stream != PrimitiveStreamMode::Sse {
            return Err(GatewayError::Protocol(
                "primitive_stream currently supports SSE stream mode only".into(),
            ));
        }

        let est_tokens = req.estimated_tokens();
        let est_cost = primitive_estimated_cost(&req);

        let lease = self
            .pool
            .acquire(est_tokens)
            .ok_or(GatewayError::NoAvailableKey)?;

        if !self.budget.try_reserve(est_cost) {
            return Err(GatewayError::BudgetExceeded);
        }

        if !lease.inner.rpm_window.try_acquire() {
            self.budget.settle(est_cost, 0);
            return Err(GatewayError::RateLimited);
        }

        let inner = match self.dispatcher.primitive_stream(&lease, &req).await {
            Ok(stream) => stream,
            Err(err) => {
                self.budget.settle(est_cost, 0);
                if matches!(
                    err,
                    ApiError::Unauthorized
                        | ApiError::RateLimited { .. }
                        | ApiError::Provider(_)
                        | ApiError::PrimitiveProvider(_)
                ) {
                    self.pool.report_error(&lease, &err);
                }
                return Err(map_api_error(err));
            }
        };

        let budget = Arc::clone(&self.budget);
        let pool = Arc::clone(&self.pool);
        let model = req.model_name().to_string();
        let stream = try_stream! {
            let mut inner = inner;
            let mut latest_usage = None;
            let mut completed = false;

            loop {
                let next = tokio::select! {
                    _ = cancel.cancelled() => {
                        Some(Err(ApiError::Cancelled))
                    }
                    item = inner.next() => item,
                };

                match next {
                    Some(Ok(event)) => {
                        match &event {
                            PrimitiveStreamEvent::Usage { usage } => {
                                latest_usage = Some(usage.clone());
                            }
                            PrimitiveStreamEvent::Completed { usage } => {
                                if let Some(usage) = usage.clone() {
                                    latest_usage = Some(usage);
                                }
                                let actual = latest_usage
                                    .as_ref()
                                    .and_then(|usage| usage.token_usage.as_ref())
                                    .map(|usage| pricing::actual(usage, &model))
                                    .unwrap_or(est_cost);
                                budget.settle(est_cost, actual);
                                pool.report_success(&lease);
                                completed = true;
                            }
                            _ => {}
                        }
                        yield event;
                    }
                    Some(Err(err)) => {
                        let observed_usage_cost = latest_usage
                            .as_ref()
                            .and_then(|usage| usage.token_usage.as_ref())
                            .map(|usage| pricing::actual(usage, &model));
                        let actual = if matches!(err, ApiError::Cancelled) {
                            observed_usage_cost.unwrap_or(0)
                        } else {
                            observed_usage_cost.unwrap_or(est_cost)
                        };
                        budget.settle(est_cost, actual);
                        if matches!(
                            err,
                            ApiError::Unauthorized
                                | ApiError::RateLimited { .. }
                                | ApiError::Provider(_)
                                | ApiError::PrimitiveProvider(_)
                        ) {
                            pool.report_error(&lease, &err);
                        }
                        Err(map_api_error(err))?;
                    }
                    None => {
                        if !completed {
                            let actual = latest_usage
                                .as_ref()
                                .and_then(|usage| usage.token_usage.as_ref())
                                .map(|usage| pricing::actual(usage, &model))
                                .unwrap_or(est_cost);
                            budget.settle(est_cost, actual);
                            pool.report_success(&lease);
                            yield PrimitiveStreamEvent::Completed { usage: latest_usage.clone() };
                        }
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    pub async fn primitive_realtime(
        &self,
        req: PrimitiveRequest,
        _cancel: CancellationToken,
    ) -> Result<PrimitiveRealtimeSession, GatewayError> {
        self.ensure_primitive_supported(&req)?;
        Err(GatewayError::Protocol(
            "primitive realtime transport is scaffolded but not implemented".into(),
        ))
    }

    fn ensure_primitive_supported(&self, req: &PrimitiveRequest) -> Result<(), GatewayError> {
        if !self.dispatcher.primitive_endpoint().supports(req) {
            return Err(GatewayError::Protocol(format!(
                "unsupported primitive endpoint {:?}/{:?}/{:?}/{:?}",
                req.provider, req.endpoint, req.wire_format, req.stream
            )));
        }
        Ok(())
    }

    pub fn pool_status(&self) -> Vec<KeyStatus> {
        self.pool.status()
    }

    pub fn budget_remaining_usd(&self) -> f64 {
        self.budget.remaining_usd()
    }

    pub fn budget_used_usd(&self) -> f64 {
        self.budget.used_usd()
    }
}

/// A builder for constructing a [`Gateway`].
pub struct GatewayBuilder {
    provider_endpoint: ProviderEndpoint,
    keys: Vec<KeyConfig>,
    budget_limit_usd: Option<f64>,
    pool_config: PoolConfig,
    request_timeout: Duration,
    primitive_endpoint: Option<PrimitiveProviderEndpoint>,
}

impl GatewayBuilder {
    pub fn new(provider_endpoint: ProviderEndpoint) -> Self {
        Self {
            provider_endpoint,
            keys: Vec::new(),
            budget_limit_usd: None,
            pool_config: PoolConfig::default(),
            request_timeout: Duration::from_secs(120),
            primitive_endpoint: None,
        }
    }

    pub fn add_key(mut self, key: KeyConfig) -> Self {
        self.keys.push(key);
        self
    }

    pub fn add_keys(mut self, keys: impl IntoIterator<Item = KeyConfig>) -> Self {
        self.keys.extend(keys);
        self
    }

    pub fn budget_limit_usd(mut self, limit: f64) -> Self {
        self.budget_limit_usd = Some(limit);
        self
    }

    pub fn pool_config(mut self, config: PoolConfig) -> Self {
        self.pool_config = config;
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn primitive_endpoint(mut self, endpoint: PrimitiveProviderEndpoint) -> Self {
        self.primitive_endpoint = Some(endpoint);
        self
    }

    pub fn build(self) -> Result<Gateway, GatewayError> {
        if self.keys.is_empty() {
            return Err(GatewayError::NoAvailableKey);
        }

        let keys: Vec<Arc<KeyInner>> = self
            .keys
            .into_iter()
            .map(|kc| Arc::new(KeyInner::new(kc.key, kc.label, kc.tpm_limit, kc.rpm_limit)))
            .collect();

        let pool = Arc::new(KeyPool::new(keys, self.pool_config));
        let budget = Arc::new(BudgetTracker::new(
            self.budget_limit_usd.unwrap_or(f64::MAX),
        ));
        let dispatcher = Arc::new(if let Some(primitive_endpoint) = self.primitive_endpoint {
            Dispatcher::new_with_primitive_endpoint(
                self.provider_endpoint,
                primitive_endpoint,
                self.request_timeout,
            )
        } else {
            Dispatcher::new(self.provider_endpoint, self.request_timeout)
        });

        Ok(Gateway {
            pool,
            budget,
            dispatcher,
        })
    }

    pub fn from_config(config: GatewayConfig) -> Result<Gateway, GatewayError> {
        let mut builder = Self::new(config.provider_endpoint)
            .add_keys(config.keys)
            .pool_config(config.pool_config)
            .request_timeout(config.request_timeout);

        if let Some(limit) = config.budget_limit_usd {
            builder = builder.budget_limit_usd(limit);
        }

        if let Some(primitive_endpoint) = config.primitive_endpoint {
            builder = builder.primitive_endpoint(primitive_endpoint);
        }

        builder.build()
    }
}

fn primitive_estimated_cost(req: &PrimitiveRequest) -> u64 {
    match req.budget_class() {
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
        | PrimitiveBudgetClass::UploadOrStorage => 0,
        PrimitiveBudgetClass::TokenMetered | PrimitiveBudgetClass::BillableUnitMetered => {
            pricing::estimate(req.estimated_tokens(), req.model_name())
        }
    }
}

fn map_api_error(error: ApiError) -> GatewayError {
    match error {
        ApiError::Unauthorized => GatewayError::Unauthorized,
        ApiError::RateLimited { .. } => GatewayError::RateLimited,
        ApiError::Cancelled => GatewayError::Cancelled,
        ApiError::Provider(error) => GatewayError::Provider(error),
        ApiError::PrimitiveProvider(error) => GatewayError::PrimitiveProvider(error),
        ApiError::Protocol(message) => GatewayError::Protocol(message),
    }
}
