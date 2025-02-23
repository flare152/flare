use crate::common::ctx::extensions::Extensions;
use crate::common::error::{FlareErr, Result};
use log::debug;
use prost::Message;
use protobuf_codegen::{Command, Platform};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct AppContext {
    remote_addr: String,
    command: Option<Command>,
    data: Vec<u8>,
    extensions: Arc<Mutex<Extensions>>,
    user_id: Option<String>,
    platform: Option<Platform>,
    client_id: Option<String>,     // 客户端标识
    language: Option<String>,
    conn_id: String,
    client_msg_id: String,
}

impl AppContext {
    // 基础信息获取
    pub fn remote_addr(&self) -> &str {
        &self.remote_addr
    }

    pub fn command(&self) -> Option<Command> {
        self.command
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub fn platform(&self) -> Option<Platform> {
        self.platform
    }

    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    // 数据操作相关
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn get_data_as<T: Message + Default>(&self) -> Result<T> {
        T::decode(&self.data[..])
            .map_err(|e| FlareErr::DecodeError(e))
    }

    // 数据类型转换
    pub fn msg_id(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| FlareErr::DecodeError(prost::DecodeError::new(e.to_string())))
    }

    pub fn bool_data(&self) -> Result<bool> {
        if self.data.is_empty() {
            return Err(FlareErr::InvalidParams("数据为空".to_string()));
        }
        Ok(self.data[0] != 0)
    }

    pub fn string_data(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| FlareErr::DecodeError(prost::DecodeError::new(e.to_string())))
    }

    pub fn int_data(&self) -> Result<i64> {
        if self.data.len() < 8 {
            return Err(FlareErr::InvalidParams("数据长度不足".to_string()));
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[0..8]);
        Ok(i64::from_le_bytes(bytes))
    }

    pub fn float_data(&self) -> Result<f64> {
        if self.data.len() < 8 {
            return Err(FlareErr::InvalidParams("数据长度不足".to_string()));
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[0..8]);
        Ok(f64::from_le_bytes(bytes))
    }

    // 扩展功能
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.extensions.lock().unwrap().contains::<T>()
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.extensions.lock().unwrap().get::<Arc<T>>().map(|v| v.clone())
    }

    pub fn set<T: Send + Sync + 'static>(&mut self, val: T) {
        self.extensions.lock().unwrap().insert(val);
    }

    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.extensions.lock().unwrap().remove()
    }

    // 生命周期管理
    pub fn destroy(&mut self) {
        debug!("Destroying AppContext for connection: {}", self.remote_addr);
        self.extensions.lock().unwrap().clear();
        self.data.clear();
        self.command = None;
        self.user_id = None;
        self.platform = None;
        self.client_id = None;
        self.language = None;
        self.conn_id = String::new();
        self.client_msg_id = String::new();
    }
}

impl Clone for AppContext {
    fn clone(&self) -> Self {
        Self {
            remote_addr: self.remote_addr.clone(),
            command: self.command.clone(),
            data: self.data.clone(),
            extensions: self.extensions.clone(),
            user_id: self.user_id.clone(),
            platform: self.platform.clone(),
            client_id: self.client_id.clone(),
            language: self.language.clone(),
            conn_id: self.conn_id.clone(),
            client_msg_id: self.client_msg_id.clone(),
        }
    }
}

#[derive(Default)]
pub struct AppContextBuilder {
    remote_addr: Option<String>,
    command: Option<Command>,
    language: Option<String>,
    data: Option<Vec<u8>>,
    extensions: Option<Arc<Mutex<Extensions>>>,
    user_id: Option<String>,
    platform: Option<Platform>,
    client_id: Option<String>,
    client_msg_id: Option<String>,
    conn_id: Option<String>,
}

impl AppContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remote_addr(mut self, addr: String) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    pub fn command(mut self, cmd: Option<Command>) -> Self {
        self.command = cmd;
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn extensions(mut self, extensions: Arc<Mutex<Extensions>>) -> Self {
        self.extensions = Some(extensions);
        self
    }

    pub fn user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn with_language(mut self, language: Option<String>) -> Self {
        match language {
            Some(lang) => self.language = Some(lang.to_string()),
            None => self.language = None,
        }
        self
    }
    pub fn with_conn_id(mut self, conn_id: String) -> Self {
        self.conn_id = Some(conn_id);
        self
    }
    pub fn with_client_msg_id(mut self, client_msg_id: String) -> Self {
        self.client_msg_id = Some(client_msg_id);
        self
    }

    pub fn build(self) -> Result<AppContext> {
        Ok(AppContext {
            remote_addr: self.remote_addr.ok_or_else(|| anyhow::anyhow!("remote_addr is required"))?,
            command: self.command,
            data: self.data.unwrap_or_default(),
            extensions: self.extensions.unwrap_or_else(|| Arc::new(Mutex::new(Extensions::new()))),
            user_id: self.user_id,
            platform: self.platform,
            client_id: self.client_id,
            language: self.language,
            conn_id: self.conn_id.unwrap_or_else(String::new),
            client_msg_id: self.client_msg_id.unwrap_or_else(String::new),
        })
    }
}
