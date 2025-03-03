use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tonic::{Request, Status};
use flare_core::context::{AppContext, AppContextBuilder};
use tonic::metadata::{MetadataValue, MetadataMap, MetadataKey};
use std::str::FromStr;

const REMOTE_ADDR_KEY: &str = "remote-addr";
const USER_ID_KEY: &str = "user-id";
const PLATFORM_KEY: &str = "platform";
const CLIENT_ID_KEY: &str = "client-id";
const LANGUAGE_KEY: &str = "language";
const CONN_ID_KEY: &str = "conn-id";
const CLIENT_MSG_ID_KEY: &str = "client-msg-id";
const VALUES_PREFIX: &str = "ctx-val-";

#[cfg(feature = "client")]
use {
    tower::{Service, Layer},
    std::future::Future,
    std::pin::Pin,
};

#[cfg(feature = "client")]
#[derive(Clone)]
pub struct AppContextConfig {
    context: Arc<Mutex<Option<AppContext>>>,
}

#[cfg(feature = "client")]
impl AppContextConfig {
    pub fn new() -> Self {
        Self {
            context: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_context(&self, context: AppContext) {
        if let Ok(mut ctx) = self.context.lock() {
            *ctx = Some(context);
        }
    }
}

#[cfg(feature = "client")]
impl Default for AppContextConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "client")]
#[derive(Clone)]
pub struct AppContextLayer {
    config: Arc<AppContextConfig>,
}

#[cfg(feature = "client")]
impl AppContextLayer {
    pub fn new(config: AppContextConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

#[cfg(feature = "client")]
impl<S> Layer<S> for AppContextLayer {
    type Service = AppContextInterceptor<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AppContextInterceptor {
            inner,
            config: self.config.clone(),
        }
    }
}

#[cfg(feature = "client")]
#[derive(Clone)]
pub struct AppContextInterceptor<S> {
    inner: S,
    config: Arc<AppContextConfig>,
}

#[cfg(feature = "client")]
impl<S, B> Service<Request<B>> for AppContextInterceptor<S>
where
    S: Service<Request<B>, Response = tonic::Response<B>, Error = Status> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = tonic::Response<B>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        if let Ok(guard) = self.config.context.lock() {
            if let Some(ctx) = guard.as_ref() {
                build_req_metadata_form_ctx(ctx, &mut request);
            }
        }

        let mut inner = self.inner.clone();
        Box::pin(async move {
            inner.call(request).await
        })
    }
}

#[cfg(feature = "client")]
pub fn build_req_metadata_form_ctx<B>(ctx: &AppContext, request: &mut Request<B>) {
    let metadata = request.metadata_mut();
    
    if let Ok(val) = MetadataValue::from_str(&ctx.remote_addr()) {
        metadata.insert(REMOTE_ADDR_KEY, val);
    }

    if let Some(user_id) = ctx.user_id() {
        if let Ok(val) = MetadataValue::from_str(&user_id) {
            metadata.insert(USER_ID_KEY, val);
        }
    }

    if let Some(platform) = ctx.platform() {
        if let Ok(val) = MetadataValue::from_str(&platform.to_string()) {
            metadata.insert(PLATFORM_KEY, val);
        }
    }

    if let Some(client_id) = ctx.client_id() {
        if let Ok(val) = MetadataValue::from_str(&client_id) {
            metadata.insert(CLIENT_ID_KEY, val);
        }
    }

    if let Some(language) = ctx.language() {
        if let Ok(val) = MetadataValue::from_str(&language) {
            metadata.insert(LANGUAGE_KEY, val);
        }
    }

    let conn_id = ctx.conn_id();
    if let Ok(val) = MetadataValue::from_str(&conn_id) {
        metadata.insert(CONN_ID_KEY, val);
    }

    let client_msg_id = ctx.client_msg_id();
    if let Ok(val) = MetadataValue::from_str(&client_msg_id) {
        metadata.insert(CLIENT_MSG_ID_KEY, val);
    }

    if let Ok(values) = ctx.values().lock() {
        for (key, value) in values.iter() {
            let metadata_key = format!("{}{}", VALUES_PREFIX, key);
            if let (Ok(key), Ok(val)) = (MetadataKey::from_bytes(metadata_key.as_bytes()), MetadataValue::try_from(value.as_str())) {
                metadata.insert(key, val);
            }
        }
    }
}

#[cfg(feature = "server")]
pub fn build_context_from_metadata(metadata: &MetadataMap) -> Result<AppContext, Status> {
    let mut builder = AppContextBuilder::new();

    if let Some(addr) = metadata.get(REMOTE_ADDR_KEY) {
        builder = builder.remote_addr(addr.to_str()
            .map_err(|_| Status::internal("Invalid remote_addr format"))?
            .to_string());
    } else {
        builder = builder.remote_addr("127.0.0.1".to_string());
    }

    if let Some(user_id) = metadata.get(USER_ID_KEY) {
        builder = builder.user_id(user_id.to_str()
            .map_err(|_| Status::internal("Invalid user_id format"))?
            .to_string());
    }

    if let Some(platform) = metadata.get(PLATFORM_KEY) {
        let platform_str = platform.to_str()
            .map_err(|_| Status::internal("Invalid platform format"))?;
        let platform_val = platform_str.parse::<i32>()
            .map_err(|_| Status::internal("Invalid platform value"))?;
        builder = builder.platform(platform_val);
    }

    if let Some(client_id) = metadata.get(CLIENT_ID_KEY) {
        builder = builder.client_id(client_id.to_str()
            .map_err(|_| Status::internal("Invalid client_id format"))?
            .to_string());
    }

    if let Some(language) = metadata.get(LANGUAGE_KEY) {
        builder = builder.with_language(Some(language.to_str()
            .map_err(|_| Status::internal("Invalid language format"))?
            .to_string()));
    }

    if let Some(conn_id) = metadata.get(CONN_ID_KEY) {
        builder = builder.with_conn_id(conn_id.to_str()
            .map_err(|_| Status::internal("Invalid conn_id format"))?
            .to_string());
    }

    if let Some(client_msg_id) = metadata.get(CLIENT_MSG_ID_KEY) {
        builder = builder.with_client_msg_id(client_msg_id.to_str()
            .map_err(|_| Status::internal("Invalid client_msg_id format"))?
            .to_string());
    }

    let values = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut values_map = values.lock()
            .map_err(|_| Status::internal("Failed to lock values"))?;
        
        for item in metadata.iter() {
            if let tonic::metadata::KeyAndValueRef::Ascii(k, v) = item {
                if k.as_str().starts_with(VALUES_PREFIX) {
                    let actual_key = k.as_str().trim_start_matches(VALUES_PREFIX);
                    let value = v.to_str()
                        .map_err(|_| Status::internal("Invalid value format"))?;
                    values_map.insert(actual_key.to_string(), value.to_string());
                }
            }
        }
    }
    builder = builder.values(values);

    builder.build()
        .map_err(|e| Status::internal(format!("Failed to build AppContext: {}", e)))
}
