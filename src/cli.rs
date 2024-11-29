use clap::{Parser, Subcommand};

#[derive(Debug, Parser, PartialEq)]
#[command(
    name = "cyborg-worker",                  // Name of the CLI tool.
    about = "A standalone off-chain worker", // Description shown in the CLI help.
    version = "1.0"                          // Version number of the CLI tool.
)]

/// `Cli` struct defines the command-line interface for the Cyborg worker.
/// This struct uses the `clap` crate to parse command-line arguments.
/// It contains a single field `command` which specifies the subcommand to be executed.
pub struct Cli {
    /// Specify the subcommand to run.
    #[command(subcommand)]
    pub command: Option<Commands>, // Defines the possible subcommands, wrapped in an `Option`.
}

// Enum to define the available subcommands. Each variant corresponds to a different command.
#[derive(Debug, Subcommand, PartialEq)]
/// `Commands` enum defines the available subcommands for the Cyborg worker.
/// Each variant represents a specific action that can be performed by the worker.
/// - `Registration`: Registers the worker with the specified API URL and account seed.
/// - `Startmining`: Starts the mining process with the specified API URL, account seed, and IPFS URL.
pub enum Commands {
    /// Register a worker with specified API URL and Account Seed.
    Registration {
        /// API URL for the registration process.
        #[clap(long, value_name = "API_URL")]
        parachain_url: String,

        /// Account Seed for the worker registration.
        #[clap(long, value_name = "ACCOUNT_SEED")]
        account_seed: String,

        /// IPFS URL for the worker.
        #[clap(long, value_name = "IPFS_URL")]
        ipfs_url: String,

        /// IPFS API KEY for the worker.
        #[clap(long, value_name = "IPFS_API_KEY")]
        ipfs_api_key: String,

        /// IPFS API SECRET for the worker.
        #[clap(long, value_name = "IPFS_API_SECRET")]
        ipfs_api_secret: String,
    },
    /// Start the worker with specified API URL and IPFS URL.
    Startmining {
        /// API URL for starting the worker
        #[clap(long, value_name = "API_URL")]
        parachain_url: String,

        /// Account ID for the worker registration.
        #[clap(long, value_name = "ACCOUNT_SEED")]
        account_seed: String,

        //// IPFS URL for the worker.
        //#[clap(long, value_name = "IPFS_URL")]
        //ipfs_url: String,
    },
}

// // Implementation block for the `Cli` struct, adding a helper function to parse arguments.
// impl Cli {
//     pub fn parse_args() -> Self {
//         // Parses command-line arguments into the `Cli` struct.
//         Cli::parse()
//     }
// }

/* 
//Unit tests
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_registration_command() {
        // Simulate running the CLI with the `registration` subcommand and arguments.
        let args = [
            "cyborg-worker",
            "registration",
            "--api-url",
            "http://example.com",
            "--account-seed",
            "12345678",
        ];

        // Parse the arguments and check if they match the expected `Cli` struct.
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(
            cli,
            Cli {
                command: Some(Commands::Registration {
                    api_url: "http://example.com".to_string(),
                    account_seed: "12345678".to_string(),
                })
            }
        );
    }

    #[test]
    fn test_start_command() {
        // Simulate running the CLI with the `start` subcommand and arguments.
        let args = [
            "cyborg-worker",
            "startmining",
            "--api-url",
            "http://example.com",
            "--account-seed",
            "12345678",
            "--ipfs-url",
            "http://ipfs.example.com",
        ];

        // Parse the arguments and verify they match the expected `Cli` struct.
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(
            cli,
            Cli {
                command: Some(Commands::Startmining {
                    api_url: "http://example.com".to_string(),
                    account_seed: "12345678".to_string(),
                    ipfs_url: "http://ipfs.example.com".to_string(),
                })
            }
        );
    }

    #[test]
    fn test_no_command() {
        // Simulate running the CLI without any subcommand.
        let args = ["cyborg-worker"];
        let cli = Cli::try_parse_from(args).unwrap();

        // Ensure `command` is `None` when no subcommand is provided.
        assert_eq!(cli, Cli { command: None });
    }

    #[test]
    fn test_invalid_command() {
        // Simulate running the CLI with an invalid subcommand.
        let args = ["cyborg-worker", "invalid"];

        // Attempt to parse the arguments and expect an error for the unrecognized command.
        let result = Cli::try_parse_from(args);
        assert!(result.is_err());
    }
}
     */