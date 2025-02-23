use std::sync::Arc;
use std::net::SocketAddr;
use std::format;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use crate::server::auth_handler::AuthHandler;
use crate::server::handlers::ServerMessageHandler;
use crate::server::server::Server;
use crate::common::error::{Result, FlareErr};
use crate::connections::{WsConnection, QuicConnection};
use crate::server::server_handler::ServerHandler;
use crate::server::sys_handler::SystemHandler;
use log::{info, error};
use crate::connections::quic_conf::create_server_config;

pub struct FlareServer<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    server: Arc<Server<S, A, Y>>,
    ws_addr: String,
    quic_addr: String,
    quic_server_name: String,
    quic_cert_path: String,
    quic_key_path: String,
}

impl<S, A, Y> FlareServer<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    pub fn new(
        ws_addr: String, 
        quic_addr: String, 
        quic_cert_path: String, 
        quic_key_path: String,
        quic_server_name: String,
        server: Server<S, A, Y>,
    ) -> Self {
        FlareServer {
            server: Arc::new(server),
            ws_addr,
            quic_addr,
            quic_server_name,
            quic_cert_path,
            quic_key_path,
        }
    }

    pub fn builder() -> FlareServerBuilder<S, A, Y> {
        FlareServerBuilder::new()
    }

    /// 运行服务器
    pub async fn run(&self) -> Result<()> {
        // 启动 WebSocket 服务器
        let ws_server = self.run_ws_server();
        // 启动 QUIC 服务器
        let quic_server = self.run_quic_server();

        // 并发运行两个服务器
        tokio::select! {
            result = ws_server => {
                if let Err(e) = result {
                    error!("WebSocket server error: {}", e);
                }
            }
            result = quic_server => {
                if let Err(e) = result {
                    error!("QUIC server error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 运行 WebSocket 服务器
    async fn run_ws_server(&self) -> Result<()> {
        let ws_addr = self.ws_addr.parse::<SocketAddr>()
            .map_err(|e| FlareErr::ConnectionError(format!("Invalid WebSocket address: {}", e)))?;
        
        let listener = TcpListener::bind(&ws_addr).await
            .map_err(|e| FlareErr::ConnectionError(format!("Failed to bind WebSocket: {}", e)))?;
        
        info!("WebSocket server listening on {}", ws_addr);
        
        let server = self.server.clone();
        
        loop {
            if let Ok((stream, addr)) = listener.accept().await {
                let server = server.clone();
                
                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            let conn = Box::new(WsConnection::new(ws_stream, addr.to_string()));
                            let _ = server.add_connection(conn).await;
                        }
                        Err(e) => error!("Failed to accept WebSocket connection: {}", e),
                    }
                });
            }
        }
    }

    /// 运行 QUIC 服务器
    async fn run_quic_server(&self) -> Result<()> {
        let quic_addr = self.quic_addr.parse::<SocketAddr>()
            .map_err(|e| FlareErr::ConnectionError(format!("Invalid QUIC address: {}", e)))?;
        let server_config = create_server_config(
            self.quic_cert_path.as_str(),
            self.quic_key_path.as_str(),
        )?;

        let endpoint = quinn::Endpoint::server(
            server_config,
            quic_addr,
        ).map_err(|e| FlareErr::ConnectionError(format!("Failed to create QUIC endpoint: {}", e)))?;

        info!("QUIC server listening on {}", quic_addr);
        
        let server = self.server.clone();
        let server_name = Arc::new(self.quic_server_name.clone());
        
        while let Some(connecting) = endpoint.accept().await {
            let server = server.clone();
            let server_name = Arc::clone(&server_name);
            
            tokio::spawn(async move {
                match connecting.await {
                    Ok(new_conn) => {
                        match QuicConnection::new(new_conn, (*server_name).clone()).await {
                            Ok(conn) => {
                                let _ = server.add_connection(Box::new(conn)).await;
                            }
                            Err(e) => error!("Failed to create QUIC connection: {}", e),
                        }
                    }
                    Err(e) => error!("Failed to accept QUIC connection: {}", e),
                }
            });
        }

        Ok(())
    }
}

pub struct FlareServerBuilder<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    ws_addr: Option<String>,
    quic_addr: Option<String>,
    quic_server_name: Option<String>,
    quic_cert_path: Option<String>,
    quic_key_path: Option<String>,
    handle: Option<ServerMessageHandler<S, A, Y>>,
}

impl<S, A, Y> FlareServerBuilder<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            ws_addr: None,
            quic_addr: None,
            quic_server_name: None,
            quic_cert_path: None,
            quic_key_path: None,
            handle: None,
        }
    }

    pub fn ws_addr(mut self, addr: impl Into<String>) -> Self {
        self.ws_addr = Some(addr.into());
        self
    }

    pub fn quic_addr(mut self, addr: impl Into<String>) -> Self {
        self.quic_addr = Some(addr.into());
        self
    }

    pub fn quic_cert_path(mut self, path: impl Into<String>) -> Self {
        self.quic_cert_path = Some(path.into());
        self
    }

    pub fn quic_key_path(mut self, path: impl Into<String>) -> Self {
        self.quic_key_path = Some(path.into());
        self
    }

    pub fn quic_server_name(mut self, name: impl Into<String>) -> Self {
        self.quic_server_name = Some(name.into());
        self
    }

    pub fn handler(mut self, handler: ServerMessageHandler<S, A, Y>) -> Self {
        self.handle = Some(handler);
        self
    }


    pub fn build(self) -> Result<FlareServer<S, A, Y>> {
        let handler = self.handle.ok_or_else(|| anyhow::anyhow!("Handler is required"))?;
        let server = Server::new(handler);
        
        Ok(FlareServer {
            server: Arc::new(server),
            ws_addr: self.ws_addr.ok_or_else(|| anyhow::anyhow!("WebSocket address is required"))?,
            quic_addr: self.quic_addr.ok_or_else(|| anyhow::anyhow!("QUIC address is required"))?,
            quic_server_name: self.quic_server_name.ok_or_else(|| anyhow::anyhow!("QUIC server name is required"))?,
            quic_cert_path: self.quic_cert_path.ok_or_else(|| anyhow::anyhow!("QUIC certificate path is required"))?,
            quic_key_path: self.quic_key_path.ok_or_else(|| anyhow::anyhow!("QUIC key path is required"))?,
        })
    }
}

impl<S, A, Y> Default for FlareServerBuilder<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}