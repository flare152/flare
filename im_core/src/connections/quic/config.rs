use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use quinn::{ClientConfig, ServerConfig, TransportConfig, VarInt};
use quinn::rustls::{RootCertStore};
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile;

/// QUIC 协议相关的常量配置
pub const MAX_CONCURRENT_BIDI_STREAMS: u32 = 100;  // 最大并发双向流数量
pub const MAX_DATA: u32 = 10 * 1024 * 1024;       // 最大数据量 (10MB)
pub const MAX_STREAM_DATA: u32 = 1 * 1024 * 1024; // 单个流最大数据量 (1MB)
pub const KEEP_ALIVE_INTERVAL: u64 = 10;           // 保活间隔 (秒)
pub const IDLE_TIMEOUT: u64 = 60;                 // 空闲超时时间 (秒)
pub const MAX_SIZE: usize = 1024 * 1024;          // 单次读写最大大小 (1MB)

/// QUIC 应用层协议标识
pub const ALPN_QUIC_HTTP: &[&str] = &["hq-29", "hugo-quic"];

/// 创建传输层配置
pub fn create_transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport
        .keep_alive_interval(Some(Duration::from_secs(KEEP_ALIVE_INTERVAL)))
        .max_idle_timeout(Some(Duration::from_secs(IDLE_TIMEOUT).try_into().unwrap()))
        .max_concurrent_bidi_streams(VarInt::from_u32(MAX_CONCURRENT_BIDI_STREAMS))
        .initial_mtu(1200)
        .min_mtu(1200)
        .mtu_discovery_config(Some(quinn::MtuDiscoveryConfig::default()))
        .max_concurrent_uni_streams(VarInt::from_u32(100))
        .send_window(MAX_DATA as u64)
        .receive_window(VarInt::from(MAX_DATA))
        .stream_receive_window(VarInt::from(MAX_STREAM_DATA))
        .datagram_receive_buffer_size(Some(MAX_STREAM_DATA as usize));

    transport
}

/// 创建客户端配置
pub fn create_client_config(cert_path: &str) -> anyhow::Result<ClientConfig> {
    // 加载证书
    let file = File::open(Path::new(&cert_path))?;
    let mut reader = BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .filter_map(|result| result.ok())
        .collect();

    // 配置 TLS
    let mut root_store = RootCertStore::empty();
    for cert in certs {
        root_store.add(cert)?;
    }

    // 创建 QUIC 配置
    let mut client_config = ClientConfig::with_root_certificates(Arc::new(root_store))?;
    client_config.transport_config(Arc::new(create_transport_config()));

    Ok(client_config)
}

/// 创建服务端配置
pub fn create_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    // 读取证书和私钥
    let (certs, key) = read_certs_from_file(cert_path, key_path)?;

    // 创建服务端配置
    let mut server_config = ServerConfig::with_single_cert(certs, key)
        .map_err(|e| anyhow::anyhow!("Failed to create server config: {}", e))?;
    
    // 配置传输设置
    server_config.transport_config(Arc::new(create_transport_config()));

    Ok(server_config)
}

/// 从文件读取证书和私钥
fn read_certs_from_file(cert: &str, key: &str) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    // 读取证书
    let cert_file = File::open(cert)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|cert| cert.into())
        .collect();

    // 读取私钥
    let key_file = File::open(key)?;
    let mut key_reader = BufReader::new(key_file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .collect::<std::io::Result<Vec<_>>>()?;

    let key = keys.first()
        .ok_or_else(|| anyhow::anyhow!("no private key found"))?;

    let key: PrivateKeyDer<'static> = key.to_owned().clone_key().into();

    Ok((certs, key))
}