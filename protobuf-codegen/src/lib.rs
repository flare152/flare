//! 自动生成的 protobuf 代码

pub mod flare_gen;

// 重新导出所有生成的类型
pub use flare_gen::flare::net::{
    Command,
    Platform,
    Message,
    ResCode,
    Response,
};