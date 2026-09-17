use std::{sync::OnceLock, time::Duration};

/// Reuse connections and bound both connection establishment and the full body download.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build(Duration::from_secs(30)))
}

fn build(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10).min(timeout))
        .timeout(timeout)
        .build()
        .expect("valid HTTP client configuration")
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn stalled_body_times_out_and_next_request_can_retry() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for stall in [true, false] {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut request = [0; 1024];
                    let _ = socket.read(&mut request).await;
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n",
                        )
                        .await
                        .unwrap();
                    if stall {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    let _ = socket.write_all(b"ok").await;
                });
            }
        });
        let client = super::build(std::time::Duration::from_millis(150));
        assert!(client
            .get(&url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap_err()
            .is_timeout());
        assert_eq!(
            client.get(&url).send().await.unwrap().text().await.unwrap(),
            "ok"
        );
        server.await.unwrap();
    }
}
