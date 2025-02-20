// This file is @generated by prost-build.
/// 请求消息
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    /// 命令
    #[prost(enumeration = "Command", tag = "1")]
    pub command: i32,
    /// 消息体
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// 客户端消息id
    #[prost(string, tag = "3")]
    pub client_id: ::prost::alloc::string::String,
}
/// 响应消息
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// 登录请求
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginReq {
    /// 用户id
    #[prost(string, tag = "1")]
    pub user_id: ::prost::alloc::string::String,
    /// 平台
    #[prost(enumeration = "Platform", tag = "2")]
    pub platform: i32,
    /// 客户端id
    #[prost(string, tag = "3")]
    pub client_id: ::prost::alloc::string::String,
    /// token
    #[prost(string, tag = "4")]
    pub token: ::prost::alloc::string::String,
}
/// 登录响应
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginResp {
    /// 用户id
    #[prost(string, tag = "1")]
    pub user_id: ::prost::alloc::string::String,
    /// 语言
    #[prost(string, tag = "2")]
    pub language: ::prost::alloc::string::String,
}
/// 设备平台
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Platform {
    Unknown = 0,
    Ios = 1,
    Android = 2,
    Windows = 3,
    /// mac os
    Osx = 4,
    /// 网页
    Web = 5,
    /// 迷你web
    MiniWeb = 6,
    /// Linux设备
    Linux = 7,
    /// 安卓平板
    Apad = 8,
    /// 苹果平板
    Ipad = 9,
}
impl Platform {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Ios => "IOS",
            Self::Android => "ANDROID",
            Self::Windows => "WINDOWS",
            Self::Osx => "OSX",
            Self::Web => "WEB",
            Self::MiniWeb => "MINI_WEB",
            Self::Linux => "LINUX",
            Self::Apad => "APAD",
            Self::Ipad => "IPAD",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNKNOWN" => Some(Self::Unknown),
            "IOS" => Some(Self::Ios),
            "ANDROID" => Some(Self::Android),
            "WINDOWS" => Some(Self::Windows),
            "OSX" => Some(Self::Osx),
            "WEB" => Some(Self::Web),
            "MINI_WEB" => Some(Self::MiniWeb),
            "LINUX" => Some(Self::Linux),
            "APAD" => Some(Self::Apad),
            "IPAD" => Some(Self::Ipad),
            _ => None,
        }
    }
}
/// 消息标识符
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Command {
    CmdUnknown = 0,
    /// 系统命令 (1-9)
    ///
    /// ping
    Ping = 1,
    /// pong
    Pong = 2,
    /// 登录
    Login = 3,
    /// 退出登录
    LoginOut = 4,
    /// 设置后台运行
    SetBackground = 5,
    /// 语言设置
    SetLanguage = 6,
    /// 强制用户下线
    KickOnline = 7,
    /// 链接关闭
    Close = 8,
    /// 客户端命令 (10-29)
    ///
    /// 客户端发送消息
    ClientSendMessage = 10,
    /// 客户端拉取消息
    ClientPullMessage = 11,
    /// 客户端发送请求
    ClientRequest = 12,
    /// 客户端确认接收
    ClientAck = 13,
    /// 服务端命令 (30-49)
    ///
    /// 服务端推送消息
    ServerPushMsg = 30,
    /// 服务端推送自定义消息
    ServerPushCustom = 31,
    /// 服务端推送通知
    ServerPushNotice = 32,
    /// 服务端推送数据
    ServerPushData = 33,
    /// 服务端确认接收
    ServerAck = 34,
    /// 服务端响应
    ServerResponse = 35,
}
impl Command {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::CmdUnknown => "CMD_UNKNOWN",
            Self::Ping => "PING",
            Self::Pong => "PONG",
            Self::Login => "LOGIN",
            Self::LoginOut => "LOGIN_OUT",
            Self::SetBackground => "SET_BACKGROUND",
            Self::SetLanguage => "SET_LANGUAGE",
            Self::KickOnline => "KICK_ONLINE",
            Self::Close => "CLOSE",
            Self::ClientSendMessage => "CLIENT_SEND_MESSAGE",
            Self::ClientPullMessage => "CLIENT_PULL_MESSAGE",
            Self::ClientRequest => "CLIENT_REQUEST",
            Self::ClientAck => "CLIENT_ACK",
            Self::ServerPushMsg => "SERVER_PUSH_MSG",
            Self::ServerPushCustom => "SERVER_PUSH_CUSTOM",
            Self::ServerPushNotice => "SERVER_PUSH_NOTICE",
            Self::ServerPushData => "SERVER_PUSH_DATA",
            Self::ServerAck => "SERVER_ACK",
            Self::ServerResponse => "SERVER_RESPONSE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CMD_UNKNOWN" => Some(Self::CmdUnknown),
            "PING" => Some(Self::Ping),
            "PONG" => Some(Self::Pong),
            "LOGIN" => Some(Self::Login),
            "LOGIN_OUT" => Some(Self::LoginOut),
            "SET_BACKGROUND" => Some(Self::SetBackground),
            "SET_LANGUAGE" => Some(Self::SetLanguage),
            "KICK_ONLINE" => Some(Self::KickOnline),
            "CLOSE" => Some(Self::Close),
            "CLIENT_SEND_MESSAGE" => Some(Self::ClientSendMessage),
            "CLIENT_PULL_MESSAGE" => Some(Self::ClientPullMessage),
            "CLIENT_REQUEST" => Some(Self::ClientRequest),
            "CLIENT_ACK" => Some(Self::ClientAck),
            "SERVER_PUSH_MSG" => Some(Self::ServerPushMsg),
            "SERVER_PUSH_CUSTOM" => Some(Self::ServerPushCustom),
            "SERVER_PUSH_NOTICE" => Some(Self::ServerPushNotice),
            "SERVER_PUSH_DATA" => Some(Self::ServerPushData),
            "SERVER_ACK" => Some(Self::ServerAck),
            "SERVER_RESPONSE" => Some(Self::ServerResponse),
            _ => None,
        }
    }
}
/// 消息响应码
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ResCode {
    /// 成功
    Success = 0,
    /// 未知错误
    UnknownCode = 1,
    /// 连接关闭
    ConnectionClosed = 2,
    /// 连接不存在
    ConnectionNotFound = 3,
    /// 解码错误
    DecodeError = 4,
    /// 编码错误
    EncodeError = 5,
    /// WebSocket错误
    WebsocketError = 6,
    /// 无效消息类型
    InvalidMessageType = 7,
    /// 业务错误
    BusinessError = 8,
    /// 协议错误
    ProtocolError = 9,
    /// 认证错误
    AuthError = 10,
    /// 未找到处理器
    NotFoundHandler = 11,
    /// 推送客户端错误
    PushToClientError = 12,
    /// 发送消息错误
    SendMessageError = 13,
    /// 无效参数
    InvalidParams = 14,
    /// 无效命令
    InvalidCommand = 15,
    /// 未授权
    Unauthorized = 16,
    /// 内部错误
    InternalError = 17,
    /// 无效状态
    InvalidState = 18,
    /// 超时
    Timeout = 19,
    /// 资源错误
    ResourceError = 20,
    /// 连接错误
    ConnectionError = 21,
    /// 参数错误
    ArgsError = 22,
}
impl ResCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::UnknownCode => "UNKNOWN_CODE",
            Self::ConnectionClosed => "CONNECTION_CLOSED",
            Self::ConnectionNotFound => "CONNECTION_NOT_FOUND",
            Self::DecodeError => "DECODE_ERROR",
            Self::EncodeError => "ENCODE_ERROR",
            Self::WebsocketError => "WEBSOCKET_ERROR",
            Self::InvalidMessageType => "INVALID_MESSAGE_TYPE",
            Self::BusinessError => "BUSINESS_ERROR",
            Self::ProtocolError => "PROTOCOL_ERROR",
            Self::AuthError => "AUTH_ERROR",
            Self::NotFoundHandler => "NOT_FOUND_HANDLER",
            Self::PushToClientError => "PUSH_TO_CLIENT_ERROR",
            Self::SendMessageError => "SEND_MESSAGE_ERROR",
            Self::InvalidParams => "INVALID_PARAMS",
            Self::InvalidCommand => "INVALID_COMMAND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::InternalError => "INTERNAL_ERROR",
            Self::InvalidState => "INVALID_STATE",
            Self::Timeout => "TIMEOUT",
            Self::ResourceError => "RESOURCE_ERROR",
            Self::ConnectionError => "CONNECTION_ERROR",
            Self::ArgsError => "ARGS_ERROR",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SUCCESS" => Some(Self::Success),
            "UNKNOWN_CODE" => Some(Self::UnknownCode),
            "CONNECTION_CLOSED" => Some(Self::ConnectionClosed),
            "CONNECTION_NOT_FOUND" => Some(Self::ConnectionNotFound),
            "DECODE_ERROR" => Some(Self::DecodeError),
            "ENCODE_ERROR" => Some(Self::EncodeError),
            "WEBSOCKET_ERROR" => Some(Self::WebsocketError),
            "INVALID_MESSAGE_TYPE" => Some(Self::InvalidMessageType),
            "BUSINESS_ERROR" => Some(Self::BusinessError),
            "PROTOCOL_ERROR" => Some(Self::ProtocolError),
            "AUTH_ERROR" => Some(Self::AuthError),
            "NOT_FOUND_HANDLER" => Some(Self::NotFoundHandler),
            "PUSH_TO_CLIENT_ERROR" => Some(Self::PushToClientError),
            "SEND_MESSAGE_ERROR" => Some(Self::SendMessageError),
            "INVALID_PARAMS" => Some(Self::InvalidParams),
            "INVALID_COMMAND" => Some(Self::InvalidCommand),
            "UNAUTHORIZED" => Some(Self::Unauthorized),
            "INTERNAL_ERROR" => Some(Self::InternalError),
            "INVALID_STATE" => Some(Self::InvalidState),
            "TIMEOUT" => Some(Self::Timeout),
            "RESOURCE_ERROR" => Some(Self::ResourceError),
            "CONNECTION_ERROR" => Some(Self::ConnectionError),
            "ARGS_ERROR" => Some(Self::ArgsError),
            _ => None,
        }
    }
}
