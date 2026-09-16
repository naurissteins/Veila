use std::{path::PathBuf, time::Duration};

use tokio::{net::UnixStream, sync::mpsc::UnboundedSender, time::timeout};
use veila_common::ipc::{
    ClientMessage, DaemonControlMessage, DaemonControlResponse, DaemonMessage,
};

use crate::adapters::ipc;

const CONNECTION_IO_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct AuthConnection {
    pub(super) generation: PathBuf,
    pub(super) stream: UnixStream,
    pub(super) message: ClientMessage,
}

pub(super) struct ControlConnection {
    pub(super) stream: UnixStream,
    pub(super) message: DaemonControlMessage,
}

pub(super) fn spawn_auth_reader(
    stream: UnixStream,
    generation: PathBuf,
    sender: UnboundedSender<AuthConnection>,
) {
    tokio::spawn(read_auth_connection(
        stream,
        generation,
        sender,
        CONNECTION_IO_TIMEOUT,
    ));
}

pub(super) fn spawn_control_reader(stream: UnixStream, sender: UnboundedSender<ControlConnection>) {
    tokio::spawn(read_control_connection(
        stream,
        sender,
        CONNECTION_IO_TIMEOUT,
    ));
}

async fn read_auth_connection(
    mut stream: UnixStream,
    generation: PathBuf,
    sender: UnboundedSender<AuthConnection>,
    io_timeout: Duration,
) {
    match timeout(io_timeout, ipc::read_client_message(&mut stream)).await {
        Ok(Ok(Some(message))) => {
            let _ = sender.send(AuthConnection {
                generation,
                stream,
                message,
            });
        }
        Ok(Ok(None)) => tracing::debug!("auth client closed without a request"),
        Ok(Err(error)) => {
            tracing::warn!("rejected invalid auth request: {error:#}");
            send_auth_error(
                &mut stream,
                "invalid or unsupported auth request",
                io_timeout,
            )
            .await;
        }
        Err(_) => {
            tracing::warn!("auth client timed out before sending a request");
            send_auth_error(&mut stream, "auth request timed out", io_timeout).await;
        }
    }
}

async fn read_control_connection(
    mut stream: UnixStream,
    sender: UnboundedSender<ControlConnection>,
    io_timeout: Duration,
) {
    match timeout(io_timeout, ipc::read_daemon_control_message(&mut stream)).await {
        Ok(Ok(Some(message))) => {
            let _ = sender.send(ControlConnection { stream, message });
        }
        Ok(Ok(None)) => tracing::debug!("control client closed without a request"),
        Ok(Err(error)) => {
            tracing::warn!("rejected invalid daemon control request: {error:#}");
            send_control_error(
                &mut stream,
                "invalid or unsupported daemon control request",
                io_timeout,
            )
            .await;
        }
        Err(_) => {
            tracing::warn!("daemon control client timed out before sending a request");
            send_control_error(&mut stream, "daemon control request timed out", io_timeout).await;
        }
    }
}

async fn send_auth_error(stream: &mut UnixStream, reason: &str, io_timeout: Duration) {
    let response = DaemonMessage::Error {
        reason: reason.to_string(),
    };
    match timeout(io_timeout, ipc::write_daemon_message(stream, &response)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::debug!("failed to send auth error response: {error:#}"),
        Err(_) => tracing::debug!("timed out sending auth error response"),
    }
}

async fn send_control_error(stream: &mut UnixStream, reason: &str, io_timeout: Duration) {
    let response = DaemonControlResponse::Error {
        reason: reason.to_string(),
    };
    match timeout(
        io_timeout,
        ipc::write_daemon_control_response(stream, &response),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::debug!("failed to send control error response: {error:#}"),
        Err(_) => tracing::debug!("timed out sending control error response"),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{UnixListener, UnixStream},
        sync::mpsc,
        time::timeout,
    };
    use veila_common::ipc::{
        ClientMessage, DaemonControlMessage, DaemonControlResponse, DaemonMessage, decode_message,
        encode_message,
    };

    use super::{read_auth_connection, read_control_connection};

    const TEST_TIMEOUT: Duration = Duration::from_millis(100);

    async fn socket_pair(label: &str) -> (UnixStream, UnixStream) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should follow Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "veila-connection-{label}-{}-{stamp}.sock",
            std::process::id()
        ));
        let listener = UnixListener::bind(&path).expect("listener should bind");
        let client = UnixStream::connect(&path)
            .await
            .expect("client should connect");
        let (server, _) = listener.accept().await.expect("server should accept");
        std::fs::remove_file(path).expect("test socket should be removable");
        (client, server)
    }

    async fn read_response<T>(stream: &mut UnixStream) -> T
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let mut bytes = Vec::new();
        stream
            .read_to_end(&mut bytes)
            .await
            .expect("response should be readable");
        decode_message(
            std::str::from_utf8(&bytes)
                .expect("response should be UTF-8")
                .trim_end(),
        )
        .expect("response should decode")
    }

    #[tokio::test]
    async fn malformed_auth_request_does_not_block_following_request() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let (mut malformed_client, malformed_server) = socket_pair("malformed-auth").await;
        let (mut valid_client, valid_server) = socket_pair("valid-auth-after-malformed").await;

        tokio::spawn(read_auth_connection(
            malformed_server,
            PathBuf::from("generation"),
            sender.clone(),
            TEST_TIMEOUT,
        ));
        malformed_client
            .write_all(b"not-json\n")
            .await
            .expect("malformed request should write");

        tokio::spawn(read_auth_connection(
            valid_server,
            PathBuf::from("generation"),
            sender,
            TEST_TIMEOUT,
        ));
        let mut payload = encode_message(&ClientMessage::Activity).expect("encode request");
        payload.push('\n');
        valid_client
            .write_all(payload.as_bytes())
            .await
            .expect("valid request should write");

        let connection = timeout(TEST_TIMEOUT, receiver.recv())
            .await
            .expect("valid request should not be blocked")
            .expect("reader channel should stay open");

        assert_eq!(connection.message, ClientMessage::Activity);
        drop(connection);
        let response: DaemonMessage = read_response(&mut malformed_client).await;
        assert!(matches!(response, DaemonMessage::Error { .. }));
    }

    #[tokio::test]
    async fn silent_auth_request_times_out_without_blocking_following_request() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let (_silent_client, silent_server) = socket_pair("silent-auth").await;
        let (mut valid_client, valid_server) = socket_pair("valid-auth-after-silent").await;

        tokio::spawn(read_auth_connection(
            silent_server,
            PathBuf::from("generation"),
            sender.clone(),
            TEST_TIMEOUT,
        ));
        tokio::spawn(read_auth_connection(
            valid_server,
            PathBuf::from("generation"),
            sender,
            TEST_TIMEOUT,
        ));
        let mut payload = encode_message(&ClientMessage::Activity).expect("encode request");
        payload.push('\n');
        valid_client
            .write_all(payload.as_bytes())
            .await
            .expect("valid request should write");

        let connection = timeout(TEST_TIMEOUT, receiver.recv())
            .await
            .expect("valid request should not be blocked")
            .expect("reader channel should stay open");

        assert_eq!(connection.message, ClientMessage::Activity);
    }

    #[tokio::test]
    async fn unknown_control_variant_gets_error_before_following_request() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let (mut unknown_client, unknown_server) = socket_pair("unknown-control").await;
        let (mut valid_client, valid_server) = socket_pair("valid-control-after-unknown").await;

        tokio::spawn(read_control_connection(
            unknown_server,
            sender.clone(),
            TEST_TIMEOUT,
        ));
        unknown_client
            .write_all(b"{\"FutureCommand\":{}}\n")
            .await
            .expect("unknown request should write");

        tokio::spawn(read_control_connection(valid_server, sender, TEST_TIMEOUT));
        let mut payload = encode_message(&DaemonControlMessage::Health).expect("encode request");
        payload.push('\n');
        valid_client
            .write_all(payload.as_bytes())
            .await
            .expect("valid request should write");

        let connection = timeout(TEST_TIMEOUT, receiver.recv())
            .await
            .expect("valid request should not be blocked")
            .expect("reader channel should stay open");

        assert_eq!(connection.message, DaemonControlMessage::Health);
        drop(connection);
        let response: DaemonControlResponse = read_response(&mut unknown_client).await;
        assert!(matches!(response, DaemonControlResponse::Error { .. }));
    }
}
