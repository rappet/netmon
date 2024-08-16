mod generated;

pub use generated::*;

pub mod topics {
    pub const TCP_ACK: &str = "tcp_ack_v1";
    pub const TLS_CLIENT_HELLO: &str = "tls_client_hello_v1";
}
