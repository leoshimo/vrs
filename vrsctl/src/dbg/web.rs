//! Temporary, read-only, loopback viewer. The browser polls a local snapshot;
//! the collector uses VRS pub/sub and periodically repairs missed notifications.
use super::*;
use std::sync::{Arc, RwLock};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
};

pub async fn run(client: &Client, filter: Filter, all: bool) -> Result<()> {
    let mut subscription = client.subscribe(vrs::debug::TOPIC.into()).await?;
    let state = Arc::new(RwLock::new(snapshot(client).await?));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let token = serde_json::to_value(lyric::Ref::unique())?
        .as_str()
        .context("reference must be a string")?
        .to_owned();
    let root = format!("/{token}/");
    let url = format!("http://{address}{root}");
    eprintln!("VRS dbg: {url}\nPress Ctrl-C to stop the viewer.");
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if let Err(error) = std::process::Command::new(opener).arg(&url).spawn() {
            eprintln!("Could not open browser: {error}. Open the URL above.");
        }
    }
    let config = serde_json::json!({"filter":filter.query,"all":all});
    let collector = async {
        loop {
            tokio::select! { _=subscription.recv()=>(), _=tokio::time::sleep(Duration::from_millis(250))=>() }
            let next = snapshot(client).await?;
            *state.write().unwrap() = next;
            // Coalesce bursts; pub/sub is a wakeup, the history is authoritative.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let server = async {
        let slots = Arc::new(Semaphore::new(16));
        loop {
            let (stream, _) = listener.accept().await?;
            let Ok(permit) = slots.clone().try_acquire_owned() else {
                drop(stream);
                continue;
            };
            let state = state.clone();
            let root = root.clone();
            let host = address.to_string();
            let config = config.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let _ = tokio::time::timeout(
                    Duration::from_secs(10),
                    serve(stream, &host, &root, &state, &config),
                )
                .await;
            });
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::select! {result=collector=>result,result=server=>result,_=tokio::signal::ctrl_c()=>Ok(())}
}

async fn serve(
    mut stream: TcpStream,
    host: &str,
    root: &str,
    state: &RwLock<Snapshot>,
    config: &serde_json::Value,
) -> Result<()> {
    let mut input = Vec::new();
    loop {
        let mut bytes = [0; 1024];
        let n = stream.read(&mut bytes).await?;
        if n == 0 {
            return Ok(());
        }
        input.extend_from_slice(&bytes[..n]);
        if input.len() > 8192 {
            return Ok(());
        }
        if input.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let request = std::str::from_utf8(&input)?;
    let mut lines = request.split("\r\n");
    let first = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>();
    let headers: HashMap<_, _> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim()))
        .collect();
    if first.len() != 3
        || first[0] != "GET"
        || headers.get("host") != Some(&host)
        || headers
            .get("origin")
            .is_some_and(|origin| *origin != format!("http://{host}"))
    {
        return response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden").await;
    }
    let Some(path) = first[1].strip_prefix(root) else {
        return response(&mut stream, "404 Not Found", "text/plain", b"Not found").await;
    };
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let (kind, body) = match path {
        "" => (
            "text/html; charset=utf-8",
            include_bytes!("web.html").to_vec(),
        ),
        "app.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("web.js").to_vec(),
        ),
        "style.css" => (
            "text/css; charset=utf-8",
            include_bytes!("web.css").to_vec(),
        ),
        "config" => ("application/json", serde_json::to_vec(config)?),
        "api" => {
            let parsed = (|| -> Result<Filter> {
                let mut query_text = String::new();
                for part in query.split('&').filter(|p| !p.is_empty()) {
                    let (key, value) = part.split_once('=').unwrap_or((part, ""));
                    anyhow::ensure!(key == "filter", "Use the filter query parameter");
                    query_text = decode(value)?;
                }
                Filter::parse(&query_text)
            })();
            let filter = match parsed {
                Ok(filter) => filter,
                Err(error) => {
                    return response(
                        &mut stream,
                        "400 Bad Request",
                        "text/plain",
                        error.to_string().as_bytes(),
                    )
                    .await
                }
            };
            let snapshot = state.read().unwrap().clone();
            (
                "application/json",
                serde_json::to_vec(&{
                    let mut selection = filter.select(&snapshot, now_ms());
                    selection.snapshot = snapshot; // Inspectors remain stable when the query changes.
                    selection
                })?,
            )
        }
        _ => return response(&mut stream, "404 Not Found", "text/plain", b"Not found").await,
    };
    response(&mut stream, "200 OK", kind, &body).await
}
fn decode(value: &str) -> Result<String> {
    let mut result = Vec::new();
    let mut bytes = value.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'%' => {
                let a = bytes.next().context("incomplete query escape")?;
                let b = bytes.next().context("incomplete query escape")?;
                let hex = std::str::from_utf8(&[a, b])?.to_owned();
                result.push(u8::from_str_radix(&hex, 16)?);
            }
            b'+' => result.push(b' '),
            byte => result.push(byte),
        }
    }
    Ok(String::from_utf8(result)?)
}
async fn response(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> Result<()> {
    let header=format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; frame-ancestors 'none'; base-uri 'none'\r\n\r\n",body.len());
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn request(path: &str, host: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let path = path.to_owned();
        let host = host.to_owned();
        let client = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            stream
                .write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut bytes = vec![];
            stream.read_to_end(&mut bytes).await.unwrap();
            String::from_utf8(bytes).unwrap()
        });
        let (stream, _) = listener.accept().await.unwrap();
        let state = RwLock::new(Snapshot {
            cursor: 0,
            dropped: 0,
            evicted: 0,
            records: vec![],
        });
        serve(
            stream,
            "127.0.0.1:1234",
            "/secret/",
            &state,
            &serde_json::json!({}),
        )
        .await
        .unwrap();
        client.await.unwrap()
    }
    #[tokio::test]
    async fn viewer_routes_are_read_only_scoped_and_do_not_accept_rebound_hosts() {
        let response = request("/secret/api", "127.0.0.1:1234").await;
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("\"records\":[]"));
        assert!(request("/secret/api?filter=file::x:0", "127.0.0.1:1234")
            .await
            .starts_with("HTTP/1.1 400"));
        assert!(request("/secret/api", "attacker.example")
            .await
            .starts_with("HTTP/1.1 403"));
        assert!(request("/api", "127.0.0.1:1234")
            .await
            .starts_with("HTTP/1.1 404"));
        let page = request("/secret/", "127.0.0.1:1234").await;
        assert!(page.contains("Content-Security-Policy:"));
        assert!(page.contains("id=\"recording\""));
    }
}
