//! HTTP REST API and Static Web Server for ZapFast Server.

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::ServerBuilder;

use crate::server::db::{ServerDb, User};
use crate::server::manager::InstanceManager;
use crate::server::web_assets::INDEX_HTML;
use crate::server::ws::handle_ws_stream;

pub async fn run_server(
    port: u16,
    db: ServerDb,
    manager: InstanceManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    log::info!("ZapFast Server listening on http://0.0.0.0:{port}");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(res) => res,
            Err(e) => {
                log::debug!("Error accepting connection: {e}");
                continue;
            }
        };

        let db_clone = db.clone();
        let mgr_clone = manager.clone();

        tokio::spawn(async move {
            handle_connection(stream, peer, db_clone, mgr_clone).await;
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    db: ServerDb,
    manager: InstanceManager,
) {
    let mut peek_buf = [0u8; 1024];
    let n = match stream.peek(&mut peek_buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let peek_str = String::from_utf8_lossy(&peek_buf[..n]);
    if peek_str.contains("Upgrade: websocket") || peek_str.contains("upgrade: websocket") {
        match ServerBuilder::new().accept(stream).await {
            Ok((_req, ws_stream)) => {
                handle_ws_stream(ws_stream, db, manager).await;
            }
            Err(e) => {
                log::debug!("WebSocket handshake error from {peer}: {e}");
            }
        }
        return;
    }

    // Handle regular HTTP request
    let mut req_buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 2048];
    loop {
        let read = match stream.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return,
        };
        req_buf.extend_from_slice(&chunk[..read]);
        if req_buf.windows(4).any(|w| w == b"\r\n\r\n") || req_buf.len() > 65536 {
            break;
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let status = match req.parse(&req_buf) {
        Ok(httparse::Status::Complete(offset)) => offset,
        _ => {
            let _ = send_response(&mut stream, 400, "text/plain", b"Bad Request").await;
            return;
        }
    };

    let method = req.method.unwrap_or("");
    let path = req.path.unwrap_or("/");
    let body = &req_buf[status..];

    // Extract Bearer token if present
    let auth_token = req.headers.iter()
        .find(|h| h.name.eq_ignore_ascii_case("authorization"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .and_then(|v| {
            if v.starts_with("Bearer ") {
                Some(v[7..].trim().to_string())
            } else {
                None
            }
        });

    let current_user: Option<User> = auth_token.as_deref().and_then(|t| db.validate_session(t).ok().flatten());

    route_http(
        &mut stream,
        method,
        path,
        body,
        current_user,
        &db,
        &manager,
    ).await;
}

async fn route_http(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    body: &[u8],
    user: Option<User>,
    db: &ServerDb,
    manager: &InstanceManager,
) {
    if method == "GET" && (path == "/" || path == "/index.html") {
        let _ = send_response(stream, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/login" {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
            let u = val.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let p = val.get("password").and_then(|v| v.as_str()).unwrap_or("");
            if let Ok(Some(authed)) = db.authenticate(u, p) {
                if let Ok(tok) = db.create_session(&authed.id) {
                    let resp = serde_json::json!({
                        "token": tok,
                        "user": authed,
                    });
                    let _ = send_response(stream, 200, "application/json", resp.to_string().as_bytes()).await;
                    return;
                }
            }
        }
        let _ = send_response(stream, 401, "application/json", r#"{"error":"Credenciais inválidas"}"#.as_bytes()).await;
        return;
    }

    // Protected endpoints
    let Some(user) = user else {
        let _ = send_response(stream, 401, "application/json", r#"{"error":"Não autenticado"}"#.as_bytes()).await;
        return;
    };

    if method == "GET" && path == "/api/me" {
        let resp = serde_json::json!({ "user": user });
        let _ = send_response(stream, 200, "application/json", resp.to_string().as_bytes()).await;
        return;
    }

    if method == "GET" && path == "/api/instances" {
        let all = manager.list_instances();
        let allowed = if user.role == "admin" {
            all
        } else {
            let perms = db.get_user_permissions(&user.id).unwrap_or_default();
            all.into_iter().filter(|i| perms.contains(&i.id)).collect()
        };
        let resp = serde_json::to_string(&allowed).unwrap_or_default();
        let _ = send_response(stream, 200, "application/json", resp.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/instances" {
        if user.role != "admin" {
            let _ = send_response(stream, 403, "application/json", br#"{"error":"Requer perfil administrador"}"#).await;
            return;
        }
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
            let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or(id);
            if !id.is_empty() {
                manager.start_instance(id, name);
                let _ = send_response(stream, 200, "application/json", br#"{"ok":true}"#).await;
                return;
            }
        }
        let _ = send_response(stream, 400, "application/json", r#"{"error":"ID inválido"}"#.as_bytes()).await;
        return;
    }

    if method == "DELETE" && path.starts_with("/api/instances/") {
        if user.role != "admin" {
            let _ = send_response(stream, 403, "application/json", br#"{"error":"Requer perfil administrador"}"#).await;
            return;
        }
        let id = &path["/api/instances/".len()..];
        manager.remove_instance(id);
        let _ = send_response(stream, 200, "application/json", br#"{"ok":true}"#).await;
        return;
    }

    if method == "GET" && path == "/api/users" {
        if let Ok(users) = db.list_users() {
            let mut list = Vec::new();
            for u in users {
                let perms = db.get_user_permissions(&u.id).unwrap_or_default();
                list.push(serde_json::json!({
                    "id": u.id,
                    "username": u.username,
                    "role": u.role,
                    "created_at": u.created_at,
                    "permissions": perms,
                }));
            }
            let resp = serde_json::to_string(&list).unwrap_or_default();
            let _ = send_response(stream, 200, "application/json", resp.as_bytes()).await;
            return;
        }
        let _ = send_response(stream, 500, "application/json", r#"{"error":"Erro ao listar usuários"}"#.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/users" {
        if user.role != "admin" {
            let _ = send_response(stream, 403, "application/json", br#"{"error":"Requer perfil administrador"}"#).await;
            return;
        }
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
            let uname = val.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let pwd = val.get("password").and_then(|v| v.as_str()).unwrap_or("");
            let role = val.get("role").and_then(|v| v.as_str()).unwrap_or("operator");
            if !uname.is_empty() && !pwd.is_empty() {
                if let Ok(created) = db.create_user(uname, pwd, role) {
                    let resp = serde_json::to_string(&created).unwrap_or_default();
                    let _ = send_response(stream, 200, "application/json", resp.as_bytes()).await;
                    return;
                }
            }
        }
        let _ = send_response(stream, 400, "application/json", r#"{"error":"Dados inválidos"}"#.as_bytes()).await;
        return;
    }

    if method == "DELETE" && path.starts_with("/api/users/") {
        if user.role != "admin" {
            let _ = send_response(stream, 403, "application/json", br#"{"error":"Requer perfil administrador"}"#).await;
            return;
        }
        let id = &path["/api/users/".len()..];
        let _ = db.delete_user(id);
        let _ = send_response(stream, 200, "application/json", br#"{"ok":true}"#).await;
        return;
    }

    if method == "PUT" && path.starts_with("/api/users/") && path.ends_with("/permissions") {
        if user.role != "admin" {
            let _ = send_response(stream, 403, "application/json", br#"{"error":"Requer perfil administrador"}"#).await;
            return;
        }
        let prefix = "/api/users/";
        let suffix = "/permissions";
        let user_id = &path[prefix.len()..path.len() - suffix.len()];
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
            if let Some(arr) = val.get("account_ids").and_then(|v| v.as_array()) {
                let ids: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                let _ = db.set_user_permissions(user_id, &ids);
                let _ = send_response(stream, 200, "application/json", br#"{"ok":true}"#).await;
                return;
            }
        }
        let _ = send_response(stream, 400, "application/json", r#"{"error":"Formato inválido"}"#.as_bytes()).await;
        return;
    }

    let _ = send_response(stream, 404, "text/plain", b"Not Found").await;
}

async fn send_response(
    stream: &mut TcpStream,
    code: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), std::io::Error> {
    let status_line = match code {
        200 => "200 OK",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        500 => "500 Internal Server Error",
        _ => "200 OK",
    };
    let headers = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}
