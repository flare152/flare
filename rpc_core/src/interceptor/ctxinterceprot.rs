use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tonic::{Request, Status};
use tonic::service::Interceptor;
use flare::context::{AppContext, AppContextBuilder};
use tonic::metadata::MetadataValue;
use std::str::FromStr;
use tonic::metadata::MetadataKey;

const REMOTE_ADDR_KEY: &str = "remote-addr";
const USER_ID_KEY: &str = "user-id";
const PLATFORM_KEY: &str = "platform";
const CLIENT_ID_KEY: &str = "client-id";
const LANGUAGE_KEY: &str = "language";
const CONN_ID_KEY: &str = "conn-id";
const CLIENT_MSG_ID_KEY: &str = "client-msg-id";
const VALUES_PREFIX: &str = "ctx-val-";

#[derive(Clone)]
pub struct AppContextInterceptor {
    context: Arc<Mutex<Option<AppContext>>>,
}

impl AppContextInterceptor {
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

impl Default for AppContextInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl Interceptor for AppContextInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let context = self.context.lock()
            .map_err(|_| Status::internal("Failed to lock context"))?;
            
        if let Some(ctx) = context.as_ref() {
            let mut metadata = request.metadata_mut();
            
            // 添加所有基本字段
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

            // 添加 conn_id 和 client_msg_id
            let conn_id = ctx.conn_id();
            if let Ok(val) = MetadataValue::from_str(&conn_id) {
                metadata.insert(CONN_ID_KEY, val);
            }

            let client_msg_id = ctx.client_msg_id();
            if let Ok(val) = MetadataValue::from_str(&client_msg_id) {
                metadata.insert(CLIENT_MSG_ID_KEY, val);
            }
            
            // 添加自定义值
            if let Ok(values) = ctx.values().lock() {
                for (key, value) in values.iter() {
                    let metadata_key = format!("{}{}", VALUES_PREFIX, key);
                    if let (Ok(key), Ok(val)) = (MetadataKey::from_bytes(metadata_key.as_bytes()), MetadataValue::try_from(value.as_str())) {
                        metadata.insert(key, val);
                    }
                }
            }
        }
        Ok(request)
    }
}

pub fn build_context_from_metadata(metadata: &tonic::metadata::MetadataMap) -> Result<AppContext, Status> {
    let mut builder = AppContextBuilder::new();

    // 从元数据中提取信息，使用 map_err 处理错误
    if let Some(addr) = metadata.get(REMOTE_ADDR_KEY) {
        builder = builder.remote_addr(addr.to_str()
            .map_err(|_| Status::internal("Invalid remote_addr format"))?
            .to_string());
    } else {
        return Err(Status::internal("remote_addr is required"));
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

    // 从元数据中提取 values，优化错误处理
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
