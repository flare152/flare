use crate::common::error::error::Result;
use protobuf_codegen::{Command, Platform};
use std::sync::Arc;
use prost::Message;

/// 应用上下文 trait
pub trait Context: Send + Sync {
    /// 获取远程地址
    fn remote_addr(&self) -> &str;

    /// 获取命令
    fn command(&self) -> Option<Command>;

    /// 获取用户ID
    fn user_id(&self) -> Option<&str>;

    /// 获取平台
    fn platform(&self) -> Option<Platform>;

    /// 获取客户端ID
    fn client_id(&self) -> Option<&str>;

    /// 获取语言设置
    fn language(&self) -> Option<&str>;

    /// 获取原始数据
    fn data(&self) -> &[u8];

    /// 设置数据
    fn set_data(&mut self, data: Vec<u8>);

    /// 获取序列化后的数据
    fn get_data_as<T: Message + Default>(&self) -> Result<T>;

    /// 获取消息ID
    fn msg_id(&self) -> Result<String>;

    /// 获取布尔值数据
    fn bool_data(&self) -> Result<bool>;

    /// 获取字符串数据
    fn string_data(&self) -> Result<String>;

    /// 获取整数数据
    fn int_data(&self) -> Result<i64>;

    /// 获取浮点数数据
    fn float_data(&self) -> Result<f64>;

    /// 获取二进制数据
    fn bytes_data(&self) -> &[u8];

    /// 检查是否包含某个扩展
    fn contains<T: Send + Sync + 'static>(&self) -> bool;

    /// 获取扩展
    fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;

    /// 设置扩展
    fn set<T: Send + Sync + 'static>(&mut self, val: T);

    /// 移除扩展
    fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T>;

    /// 销毁上下文
    fn destroy(&mut self);
}

pub trait TypeContext {
    fn contains<T: Send + Sync + 'static>(&self) -> bool;
    fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;
    fn set<T: Send + Sync + 'static>(&mut self, val: T);
    fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T>;
}