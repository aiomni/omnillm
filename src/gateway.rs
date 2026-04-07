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
use crate::protocol::ProviderEndpoint;
use crate::types::{LlmRequest, LlmResponse, LlmStreamEvent, Message, MessageRole};

pub type GatewayStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, GatewayError>> + Send>>;

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
            | Err(ApiError::Provider(_)) => {
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
                    ApiError::Unauthorized | ApiError::RateLimited { .. } | ApiError::Provider(_)
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
                            LlmStreamEvent::ResponseStarted { .. } => {}
                            LlmStreamEvent::TextDelta { delta } => {
                                content.push_str(delta);
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
                        budget.settle(est_cost, 0);
                        if matches!(
                            err,
                            ApiError::Unauthorized | ApiError::RateLimited { .. } | ApiError::Provider(_)
                        ) {
                            pool.report_error(&lease, &err);
                        }
                        Err(map_api_error(err))?;
                    }
                    None => {
                        if !completed {
                            let usage = usage.unwrap_or_default();
                            let actual = pricing::actual(&usage, &req.model);
                            budget.settle(est_cost, actual);
                            pool.report_success(&lease);
                            let response = LlmResponse::from_message(
                                provider_protocol,
                                model.clone(),
                                Message::text(MessageRole::Assistant, content.clone()),
                                usage,
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
}

impl GatewayBuilder {
    pub fn new(provider_endpoint: ProviderEndpoint) -> Self {
        Self {
            provider_endpoint,
            keys: Vec::new(),
            budget_limit_usd: None,
            pool_config: PoolConfig::default(),
            request_timeout: Duration::from_secs(120),
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
        let dispatcher = Arc::new(Dispatcher::new(
            self.provider_endpoint,
            self.request_timeout,
        ));

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

        builder.build()
    }
}

fn map_api_error(error: ApiError) -> GatewayError {
    match error {
        ApiError::Unauthorized => GatewayError::Unauthorized,
        ApiError::RateLimited { .. } => GatewayError::RateLimited,
        ApiError::Cancelled => GatewayError::Cancelled,
        ApiError::Provider(error) => GatewayError::Provider(error),
        ApiError::Protocol(message) => GatewayError::Protocol(message),
    }
}
