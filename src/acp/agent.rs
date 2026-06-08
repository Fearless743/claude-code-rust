use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum AcpRequest {
    Initialize {
        id: u64,
        params: InitializeParams,
    },
    Prompt {
        id: u64,
        session_id: String,
        params: PromptParams,
    },
    ListSessions {
        id: u64,
    },
    Cancel {
        id: u64,
        session_id: String,
    },
    CloseSession {
        id: u64,
        session_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub client_info: ClientInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptParams {
    pub prompt: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum AcpResponse {
    InitializeResult {
        id: u64,
        result: Value,
    },
    SessionUpdate {
        session_id: String,
        update: SessionUpdate,
    },
    Error {
        id: u64,
        error: ErrorDetail,
    },
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub update_type: String,
    pub content: Vec<UpdateContent>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum UpdateContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

pub fn write_response<W: Write>(writer: &mut W, response: &AcpResponse) -> eyre::Result<()> {
    let json = serde_json::to_string(response)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;
    Ok(())
}

pub fn read_request<R: BufRead>(reader: &mut R) -> eyre::Result<AcpRequest> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(serde_json::from_str(&line)?)
}

pub async fn run_acp_agent() -> eyre::Result<()> {
    use std::io;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut sessions: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    loop {
        let request = match read_request(&mut reader) {
            Ok(r) => r,
            Err(_) => break,
        };

        match request {
            AcpRequest::Initialize { id, .. } => {
                write_response(
                    &mut stdout,
                    &AcpResponse::InitializeResult {
                        id,
                        result: serde_json::json!({
                            "protocolVersion": "1.0",
                            "capabilities": {"tools": true, "streaming": true}
                        }),
                    },
                )?;
            }
            AcpRequest::Prompt {
                id,
                session_id,
                params,
            } => {
                sessions.insert(session_id, params.prompt);
                write_response(
                    &mut stdout,
                    &AcpResponse::SessionUpdate {
                        session_id: "".into(),
                        update: SessionUpdate {
                            update_type: "text".into(),
                            content: vec![UpdateContent::Text {
                                text: "ACP agent ready".into(),
                            }],
                        },
                    },
                )?;
            }
            AcpRequest::ListSessions { id } => {
                write_response(
                    &mut stdout,
                    &AcpResponse::InitializeResult {
                        id,
                        result: serde_json::json!({"sessions": []}),
                    },
                )?;
            }
            _ => {
                write_response(
                    &mut stdout,
                    &AcpResponse::Error {
                        id: 0,
                        error: ErrorDetail {
                            code: -32601,
                            message: "Not implemented".into(),
                        },
                    },
                )?;
            }
        }
    }
    Ok(())
}
