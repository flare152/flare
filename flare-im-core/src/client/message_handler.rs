use async_trait::async_trait;
use log::debug;
use flare_core::flare_net::net::{Command, Response};

#[async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    /// 处理消息
    async fn on_message(&self, msg: Vec<u8>);

    /// 处理自定义消息
    async fn on_custom_message(&self, msg: Vec<u8>);

    /// 处理通知消息
    async fn on_notice_message(&self, msg: Vec<u8>);

    /// 处理响应
    async fn on_response(&self, msg: &Response);
    /// 处理服务端的ack
    async fn on_ack_message(&self, msg: Vec<u8>);
    /// 处理数据消息
    async fn on_data(&self, data: Vec<u8>);
    /// 获取支持的命令列表
    fn supported_commands(&self) -> Vec<Command>{
        vec![ Command::ServerPushMsg , Command::ServerPushCustom ,
            Command::ServerPushNotice , Command::ServerPushData,Command::ServerResponse,Command::ServerAck]
    }

    /// 检查是否支持某个命令
    fn supports_command(&self, command: Command) -> bool {
        self.supported_commands().contains(&command)
    }
}

#[derive(Default)]
pub struct DefMessageHandler;

impl DefMessageHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for DefMessageHandler {
    async fn on_message(&self, msg: Vec<u8>){
        debug!("收到消息: {} bytes", msg.len());
        
    }

    async fn on_custom_message(&self, msg: Vec<u8>) {
        debug!("收到自定义消息: {} bytes", msg.len());
        
    }

    async fn on_notice_message(&self, msg: Vec<u8>) {
        debug!("收到通知消息: {} bytes", msg.len());
        
    }

    async fn on_response(&self, msg: &Response) {
        debug!("收到响应: {:?}", msg);
    }

    async fn on_ack_message(&self, msg: Vec<u8>) {
        debug!("收到ack消息: {:?}", msg);
    }

    async fn on_data(&self, data: Vec<u8>){
        debug!("收到数据: {} bytes", data.len());
        
    }
}