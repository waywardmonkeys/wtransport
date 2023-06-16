use crate::driver::result::DriverError;
use crate::driver::utils::varint_q2w;
use std::fmt::Display;
use wtransport_proto::error::ErrorCode;
use wtransport_proto::varint::VarInt;

/// An enumeration representing various errors that can occur during a WebTransport connection.
#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    /// The connection was aborted by the peer (protocol level).
    #[error("Connection aborted by peer: {0}")]
    ConnectionClosed(ConnectionClose),

    /// The connection was closed by the peer (application level).
    #[error("Connection closed by peer: {0}")]
    ApplicationClosed(ApplicationClose),

    /// The connection was locally closed.
    #[error("Connection locally closed")]
    LocallyClosed,

    /// The connection was locally closed because an HTTP3 protocol violation.
    #[error("Connection locally aborted: {0}")]
    LocalH3Error(H3Error),

    /// The connection timed out.
    #[error("Connection timed out")]
    TimedOut,

    /// The connection was closed because a QUIC protocol error.
    #[error("QUIC protocol error")]
    QuicProto,
}

impl ConnectionError {
    pub(crate) fn with_driver_error(
        driver_error: DriverError,
        quic_connection: &quinn::Connection,
    ) -> Self {
        match driver_error {
            DriverError::Proto(error_code) => {
                ConnectionError::LocalH3Error(H3Error { code: error_code })
            }
            DriverError::NotConnected => quic_connection
                .close_reason()
                .expect("QUIC connection is still alive on close-cast")
                .into(),
        }
    }
}

/// An error that arise from writing to a stream.
#[derive(thiserror::Error, Debug)]
pub enum StreamWriteError {
    /// Connection has been dropped.
    #[error("Not connected")]
    NotConnected,

    /// The peer is no longer accepting data on this stream.
    #[error("Stream stopped (code: {0})")]
    Stopped(VarInt),

    /// QUIC protocol error.
    #[error("QUIC protocol error")]
    QuicProto,
}

/// An error that arise from reading from a stream.
#[derive(thiserror::Error, Debug)]
pub enum StreamReadError {
    /// Connection has been dropped.
    #[error("Not connected")]
    NotConnected,

    /// The peer abandoned transmitting data on this stream
    #[error("Stream reset (code: {0})")]
    Reset(VarInt),

    /// QUIC protocol error.
    #[error("QUIC protocol error")]
    QuicProto,
}

/// An error that arise from reading from a stream.
#[derive(thiserror::Error, Debug)]
pub enum StreamReadExactError {
    /// The stream finished before all bytes were read.
    #[error("Stream finished too early")]
    FinishedEarly,

    /// A read error occurred.
    #[error(transparent)]
    Read(StreamReadError),
}

/// An error that arise from sending a datagram.
#[derive(thiserror::Error, Debug)]
pub enum SendDatagramError {
    /// Connection has been dropped.
    #[error("Not connected")]
    NotConnected,

    /// The peer does not support receiving datagram frames.
    #[error("Peer does not support datagrams")]
    UnsupportedByPeer,

    /// The datagram is larger than the connection can currently accommodate.
    #[error("Datagram payload too large")]
    TooLarge,
}

/// An error that arise when opening a new stream.
#[derive(thiserror::Error, Debug)]
pub enum StreamOpeningError {
    /// Connection has been dropped.
    #[error("Not connected")]
    NotConnected,

    /// The peer refused the stream, stopping it during initialization.
    #[error("Opening stream refused")]
    Refused,
}

/// Reason given by an application for closing the connection
#[derive(Debug)]
pub struct ApplicationClose {
    code: VarInt,
    reason: Box<[u8]>,
}

impl Display for ApplicationClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.reason.is_empty() {
            self.code.fmt(f)?;
        } else {
            f.write_str(&String::from_utf8_lossy(&self.reason))?;
            f.write_str(" (code ")?;
            self.code.fmt(f)?;
            f.write_str(")")?;
        }
        Ok(())
    }
}

/// Reason given by the transport for closing the connection.
#[derive(Debug)]
pub struct ConnectionClose(quinn::ConnectionClose);

impl Display for ConnectionClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A struct representing an error in the HTTP3 layer.
#[derive(Debug)]
pub struct H3Error {
    code: ErrorCode,
}

impl Display for H3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.code.fmt(f)
    }
}

impl From<quinn::ConnectionError> for ConnectionError {
    fn from(error: quinn::ConnectionError) -> Self {
        match error {
            quinn::ConnectionError::VersionMismatch => ConnectionError::QuicProto,
            quinn::ConnectionError::TransportError(_) => ConnectionError::QuicProto,
            quinn::ConnectionError::ConnectionClosed(close) => {
                ConnectionError::ConnectionClosed(ConnectionClose(close))
            }
            quinn::ConnectionError::ApplicationClosed(close) => {
                ConnectionError::ApplicationClosed(ApplicationClose {
                    code: varint_q2w(close.error_code),
                    reason: close.reason.to_vec().into_boxed_slice(),
                })
            }
            quinn::ConnectionError::Reset => ConnectionError::QuicProto,
            quinn::ConnectionError::TimedOut => ConnectionError::TimedOut,
            quinn::ConnectionError::LocallyClosed => ConnectionError::LocallyClosed,
        }
    }
}
