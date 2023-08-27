use std::process::ExitCode;

use clap::{value_parser, Arg, Command, ValueHint};
use rand::seq::SliceRandom;
use rand::thread_rng;
use thiserror::Error;
use tonic::transport::Uri;
use ua_detect_validate::{
    validate_client::ValidateClient, ValidateUserAgentRequest, ValidateUserAgentResponse, Validity,
};

#[derive(Clone)]
enum UserAgent {
    Chrome,
    Firefox,
    Safari,
    Opera,
    Other(String),
}

// TODO: The ValueEnum stuff in clap doesn't allow for catch-all so we fake it
impl From<String> for UserAgent {
    fn from(s: String) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "chrome" => Self::Chrome,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            "opera" => Self::Opera,
            _ => Self::Other(s),
        }
    }
}

#[derive(Debug, Error)]
enum ClientError {
    #[error("Invalid values specified for `--url`")]
    InvalidUrl,

    #[error("Missing scheme in `--url` - try adding `http://` or `https://`")]
    MissingScheme,
}

fn get_arguments() -> Result<(Uri, String), ClientError> {
    let args = Command::new("ua-detect-client")
        .author("Blaine Bublitz <blaine.bublitz@gmail.com>")
        .version("0.0.0")
        .about("Communicate with the ua-detect gRPC server")
        .arg(
            Arg::new("url")
                .long("url")
                .help("The url to the running ua-detect service")
                .value_hint(ValueHint::Url)
                .value_parser(value_parser!(Uri))
                .default_value("http://[::1]:9001"),
        )
        .arg(Arg::new("user-agent").value_parser(value_parser!(UserAgent)).help("The full user-agent to validate or a shorthand helper [chrome, firefox, safari, opera]"))
        .get_matches();

    let url = args.get_one::<Uri>("url").ok_or(ClientError::InvalidUrl)?;
    if url.scheme().is_none() {
        return Err(ClientError::MissingScheme);
    }
    let user_agent = match args.get_one::<UserAgent>("user-agent") {
        Some(UserAgent::Chrome) => fakeit::user_agent::chrome(),
        Some(UserAgent::Firefox) => fakeit::user_agent::firefox(),
        Some(UserAgent::Safari) => fakeit::user_agent::safari(),
        Some(UserAgent::Opera) => fakeit::user_agent::opera(),
        Some(UserAgent::Other(user_agent)) => user_agent.clone(),
        None => {
            let mut rng = thread_rng();
            let mut user_agent = [
                fakeit::user_agent::opera(),
                fakeit::user_agent::chrome(),
                fakeit::user_agent::firefox(),
                fakeit::user_agent::safari(),
            ];
            // TODO: The shuffle + clone are currently working around a lifetime issue that I didn't look into
            user_agent.shuffle(&mut rng);
            user_agent[0].clone()
        }
    };

    Ok((url.clone(), user_agent))
}

#[tokio::main]
async fn main() -> ExitCode {
    // TODO(#4): Come up with an abstraction that could "unwrap" errors but still print the error messages we want
    // TODO: Abstract the various Result returning functions so they return our `ClientError` types instead of hardcoding the error messages
    // This pyramid was needed because the rust runtime debug prints the message instead of display printing it for Result return types
    match get_arguments() {
        Ok((url, user_agent)) => match ValidateClient::connect(url).await {
            Ok(mut validator) => {
                let request = tonic::Request::new(ValidateUserAgentRequest {
                    user_agent: user_agent.clone().into(),
                });

                match validator.user_agent(request).await {
                    Ok(response) => {
                        let ValidateUserAgentResponse { validity } = response.into_inner();
                        match Validity::from_i32(validity) {
                            Some(validity) => {
                                match validity {
                                    Validity::Valid => println!("{user_agent} => Valid"),
                                    Validity::Invalid => println!("{user_agent} => Invalid"),
                                    Validity::Unknown => println!("{user_agent} => Unknown"),
                                }
                                ExitCode::SUCCESS
                            }
                            None => {
                                eprintln!("Error: Invalid response from the endpoint");
                                ExitCode::FAILURE
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: Encountered a gRPC error while communicating with server - {err}");
                        ExitCode::FAILURE
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: Failed to connect to server - {err}");
                ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}
