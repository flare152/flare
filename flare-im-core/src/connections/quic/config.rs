use anyhow::Context;
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig, ServerConfig, TransportConfig, VarInt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::Arc;
use std::time::Duration;
use std::fs;
use std::path::Path;
use log::debug;
use rustls_pemfile;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::SignatureScheme;
use rustls::pki_types::UnixTime;
use rustls::RootCertStore;

/// 添加初始化函数
pub fn init_crypto() -> anyhow::Result<()> {
    // 如果已经安装了加密提供程序，就直接返回
    if rustls::crypto::ring::default_provider().install_default().is_err() {
        debug!("Crypto provider already installed");
    }
    Ok(())
}

/// QUIC 协议相关的常量配置
pub const MAX_CONCURRENT_BIDI_STREAMS: u32 = 100;  // 最大并发双向流数量
pub const MAX_DATA: u32 = 10 * 1024 * 1024;       // 最大数据量 (10MB)
pub const MAX_STREAM_DATA: u32 = 1 * 1024 * 1024; // 单个流最大数据量 (1MB)
pub const KEEP_ALIVE_INTERVAL: u64 = 10;           // 保活间隔 (秒)
pub const IDLE_TIMEOUT: u64 = 60;                 // 空闲超时时间 (秒)
pub const MAX_SIZE: usize = 1024 * 1024;          // 单次读写最大大小 (1MB)

/// QUIC 应用层协议标识
pub const ALPN_QUIC_HTTP: &[&str] = &["hq-29", "flare-quic"];

/// 创建传输层配置
pub fn create_transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport
        .keep_alive_interval(Some(Duration::from_secs(10)))
        .max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()))
        .max_concurrent_bidi_streams(VarInt::from_u32(32))
        .initial_mtu(1200)
        .min_mtu(1200)
        .max_concurrent_uni_streams(VarInt::from_u32(32))
        .stream_receive_window(VarInt::from_u32(10_000_000))
        .receive_window(VarInt::from_u32(10_000_000));

    transport
}

/// 创建客户端配置
pub fn create_client_config(cert_path: &str, is_test: bool) -> anyhow::Result<ClientConfig> {
    init_crypto()?;
    
    let client_crypto = if is_test {
        #[derive(Debug)]
        struct SkipVerifier;
        impl ServerCertVerifier for SkipVerifier {
            fn verify_server_cert(
                &self,
                _end_entity: &CertificateDer<'_>,
                _intermediates: &[CertificateDer<'_>],
                _server_name: &rustls::pki_types::ServerName<'_>,
                _ocsp: &[u8],
                _now: UnixTime,
            ) -> Result<ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                vec![
                    SignatureScheme::RSA_PSS_SHA256,
                    SignatureScheme::RSA_PSS_SHA384,
                    SignatureScheme::RSA_PSS_SHA512,
                    SignatureScheme::RSA_PKCS1_SHA256,
                    SignatureScheme::RSA_PKCS1_SHA384,
                    SignatureScheme::RSA_PKCS1_SHA512,
                    SignatureScheme::ECDSA_NISTP384_SHA384,
                    SignatureScheme::ECDSA_NISTP256_SHA256,
                    SignatureScheme::ED25519,
                ]
            }
        }

        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth();
        config.dangerous()
            .set_certificate_verifier(Arc::new(SkipVerifier));
        config.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
        config
    } else {
        // 生产环境：使用正常证书验证
        let mut roots = rustls::RootCertStore::empty();
        if !cert_path.is_empty() {
            roots.add(CertificateDer::from(fs::read(cert_path)?))?;
        }
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
        config
    };

    let mut client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
    client_config.transport_config(Arc::new(create_transport_config()));
    
    Ok(client_config)
}

/// 创建服务端配置
pub fn create_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    // 确保加密提供程序已初始化
    init_crypto()?;
    
    // 读取证书和私钥
    let key = fs::read(key_path).context("failed to read private key")?;
    let key = if Path::new(key_path).extension()
        .and_then(|x| x.to_str())
        .map_or(false, |x| x == "der") 
    {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))
    } else {
        rustls_pemfile::private_key(&mut &*key)
            .context("malformed PKCS #1 private key")?
            .ok_or_else(|| anyhow::Error::msg("no private keys found"))?
    };

    let cert_chain = fs::read(cert_path).context("failed to read certificate chain")?;
    let cert_chain = if Path::new(cert_path).extension()
        .and_then(|x| x.to_str())
        .map_or(false, |x| x == "der") 
    {
        vec![CertificateDer::from(cert_chain)]
    } else {
        rustls_pemfile::certs(&mut &*cert_chain)
            .collect::<Result<_, _>>()
            .context("invalid PEM-encoded certificate")?
    };

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;
    server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    
    // 创建服务端配置
    let mut server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));
    // 配置传输设置
    server_config.transport_config(Arc::new(create_transport_config()));

    Ok(server_config)
}

