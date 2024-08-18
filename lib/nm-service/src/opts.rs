use clap::{Parser, ValueEnum};

/// Opts that should be included in libraries.
/// Do not use directly.
///
/// #Example
///
/// ```
/// # use clap::Parser;
/// use nm_service::LibOpts;
///
/// #[derive(Parser, Debug)]
/// #[command(version, about)]
/// pub(crate) struct Opts {
///     /// Some service specific command
///     #[arg(long, env, default_value = "fnord")]
///     pub(crate) foo: String,
///
///     // Used as parameter for nm-service
///     #[command(flatten)]
///     pub(crate) lib_opts: LibOpts,
/// }
/// ```
#[derive(Parser, Debug, Clone, PartialEq)]
pub struct LibOpts {
    /// Which type of producer to use
    #[cfg(feature = "producer")]
    #[arg(long, env, default_value = "file")]
    pub producer_type: ProducerType,

    /// Comma separated list of bootstrap servers
    #[cfg(feature = "kafka")]
    #[arg(long, env, default_value = "localhost:9094")]
    pub kafka_bootstrap_servers: String,

    /// Limits the time the producer maximal waits for sending the message
    #[cfg(all(feature = "kafka", feature = "producer"))]
    #[arg(long, env, default_value = "5000")]
    pub kafka_message_timeout_ms: u32,
}

#[derive(Default, ValueEnum, Debug, Copy, Clone, PartialEq)]
pub enum ProducerType {
    /// Print in human readable form to logger
    Log,
    /// Write as JSON documents, separated by line, to a specific file
    #[default]
    File,
    /// Connect to a Kafka cluster and send Protobuf
    #[cfg(feature = "kafka")]
    Kafka,
}
