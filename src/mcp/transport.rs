use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;

use super::types::{
    CallToolParams, CallToolResult, ClientCapabilities, ClientInfo, InitializeParams,
    InitializeResult, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, ListToolsResult,
};
use serde_json::Value;

#[derive(Debug)]
pub enum McpTransportError {
    SpawnFailed(String),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    ProtocolError(String),
}

impl std::fmt::Display for McpTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed(s) => write!(f, "Spawn failed: {s}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::SerdeError(e) => write!(f, "JSON error: {e}"),
            Self::ProtocolError(s) => write!(f, "Protocol error: {s}"),
        }
    }
}

impl std::error::Error for McpTransportError {}

pub struct StdioTransport {
    process: Child,
    next_id: u64,
}

impl StdioTransport {
    pub fn spawn(command: &str, args: &[String]) -> Result<Self, McpTransportError> {
        let process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| McpTransportError::SpawnFailed(format!("{command}: {e}")))?;

        Ok(Self {
            process,
            next_id: 1,
        })
    }

    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse, McpTransportError> {
        let id = self.new_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&request).map_err(McpTransportError::SerdeError)?;

        if let Some(ref mut stdin) = self.process.stdin {
            writeln!(stdin, "{json}").map_err(McpTransportError::IoError)?;
            stdin.flush().map_err(McpTransportError::IoError)?;
        }

        // Read response
        if let Some(ref mut stdout) = self.process.stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            if let Some(line) = lines.next() {
                let line = line.map_err(McpTransportError::IoError)?;
                let msg: JsonRpcMessage =
                    serde_json::from_str(&line).map_err(McpTransportError::SerdeError)?;
                match msg {
                    JsonRpcMessage::Response(resp) => {
                        if resp.id != id {
                            return Err(McpTransportError::ProtocolError("ID mismatch".into()));
                        }
                        Ok(resp)
                    }
                    _ => Err(McpTransportError::ProtocolError("Expected response".into())),
                }
            } else {
                Err(McpTransportError::ProtocolError("No response".into()))
            }
        } else {
            Err(McpTransportError::ProtocolError("No stdout".into()))
        }
    }

    pub fn send_notification(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpTransportError> {
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let json = serde_json::to_string(&notif).map_err(McpTransportError::SerdeError)?;
        if let Some(ref mut stdin) = self.process.stdin {
            writeln!(stdin, "{json}").map_err(McpTransportError::IoError)?;
            stdin.flush().map_err(McpTransportError::IoError)?;
        }
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// High-level MCP client over stdio transport
pub struct McpClient {
    transport: StdioTransport,
    server_info: Option<super::types::ServerInfo>,
    tools: Vec<super::types::McpToolDef>,
}

impl McpClient {
    pub fn connect(command: &str, args: &[String]) -> Result<Self, McpTransportError> {
        let mut transport = StdioTransport::spawn(command, args)?;

        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {},
            client_info: ClientInfo {
                name: "claude-code-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let resp =
            transport.send_request("initialize", Some(serde_json::to_value(params).unwrap()))?;

        let init_result: InitializeResult = serde_json::from_value(resp.result.unwrap_or_default())
            .map_err(McpTransportError::SerdeError)?;

        transport.send_notification("notifications/initialized", None)?;

        Ok(Self {
            transport,
            server_info: Some(init_result.server_info),
            tools: Vec::new(),
        })
    }

    pub fn list_tools(&mut self) -> Result<Vec<super::types::McpToolDef>, McpTransportError> {
        let resp = self.transport.send_request("tools/list", None)?;
        let result: ListToolsResult = serde_json::from_value(resp.result.unwrap_or_default())
            .map_err(McpTransportError::SerdeError)?;
        self.tools = result.tools.clone();
        Ok(result.tools)
    }

    pub fn call_tool(
        &mut self,
        name: &str,
        args: Value,
    ) -> Result<CallToolResult, McpTransportError> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments: args,
        };
        let resp = self
            .transport
            .send_request("tools/call", Some(serde_json::to_value(params).unwrap()))?;
        let result: CallToolResult = serde_json::from_value(resp.result.unwrap_or_default())
            .map_err(McpTransportError::SerdeError)?;
        Ok(result)
    }

    pub fn tools(&self) -> &[super::types::McpToolDef] {
        &self.tools
    }

    pub fn server_info(&self) -> Option<&super::types::ServerInfo> {
        self.server_info.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_spawn_cat() {
        // Basic sanity test
        let result = StdioTransport::spawn("cat", &[]);
        assert!(result.is_ok());
    }
}
