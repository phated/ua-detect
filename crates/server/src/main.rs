use std::net::SocketAddr;
use std::process::ExitCode;

use clap::{Arg, Command};
use thiserror::Error;
use tonic::{transport::Server, Request, Response, Status};
use ua_detect_validate::{
    validate_server::{Validate, ValidateServer},
    ValidateUserAgentRequest, ValidateUserAgentResponse, Validity,
};
use uaparser::{Parser, UserAgentParser};

#[derive(Debug, Error)]
enum ServerError {
    #[error("Could not construct UserAgentParser: {0}")]
    UserAgentParser(uaparser::Error),

    #[error("An error occurred with gRPC: {0}")]
    GrpcStatus(#[from] Status),

    #[error("An error occurred with tonic: {0}")]
    Tonic(#[from] tonic::transport::Error),

    #[error("Invalid value specified for `--ip`")]
    InvalidIpArgument,

    #[error("Invalid value specified for `--port`")]
    InvalidPortArgument,

    #[error("Could not parse socket address: {0}")]
    InvalidSocketAddress(String),
}

impl From<uaparser::Error> for ServerError {
    fn from(value: uaparser::Error) -> Self {
        ServerError::UserAgentParser(value)
    }
}

#[derive(Debug)]
struct ValidatorService {
    ua_parser: UserAgentParser,
}

impl ValidatorService {
    /// Constructs a new [ValidatorService].
    ///
    /// Fails if a [UserAgentParser] cannot be constructed.
    fn new() -> Result<Self, ServerError> {
        // TODO: It would be cool if this could be done via `lazy_static` so it could only fail at compile-time
        let ua_parser = uaparser::UserAgentParser::from_bytes(include_bytes!("regexes.yaml"))?;
        Ok(Self { ua_parser })
    }
}

#[tonic::async_trait]
impl Validate for ValidatorService {
    async fn user_agent(
        &self,
        request: Request<ValidateUserAgentRequest>,
    ) -> Result<Response<ValidateUserAgentResponse>, Status> {
        let ValidateUserAgentRequest { user_agent } = request.into_inner();

        let client = &self.ua_parser.parse(&user_agent);

        // TODO: This would be so much nicer if families were enums
        let validity = if client.user_agent.family == "Safari" {
            Validity::Invalid
        } else if client.user_agent.family == "Firefox" {
            Validity::Valid
        } else {
            Validity::Unknown
        };

        let reply = ValidateUserAgentResponse {
            validity: validity.into(),
        };

        Ok(Response::new(reply))
    }
}

fn get_address() -> Result<SocketAddr, ServerError> {
    let args = Command::new("ua-detect-server")
        .author("Blaine Bublitz <blaine.bublitz@gmail.com>")
        .version("0.0.0")
        .about("Start a gRPC server to validate user agents")
        .arg(
            Arg::new("ip")
                .long("ip")
                .help("The IP address the server will bind to - e.g. 127.0.0.1")
                .default_value("[::1]"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .help("The port the server will listen on")
                .default_value("9001"),
        )
        .get_matches();
    let port = args
        .get_one::<String>("port")
        .ok_or(ServerError::InvalidPortArgument)?;
    let ip = args
        .get_one::<String>("ip")
        .ok_or(ServerError::InvalidIpArgument)?;
    let addr = format!("{ip}:{port}");
    let addr = addr
        .parse()
        .map_err(|_| ServerError::InvalidSocketAddress(addr))?;

    Ok(addr)
}

#[tokio::main]
async fn main() -> ExitCode {
    // TODO: Come up with an abstraction that could "unwrap" errors but still print the error messages we want
    // This pyramid was needed because the rust runtime debug prints the message instead of display printing it for Result return types
    match get_address() {
        Ok(addr) => match ValidatorService::new() {
            Ok(validator) => {
                let server = Server::builder()
                    .add_service(ValidateServer::new(validator))
                    .serve(addr);

                println!("Starting long-running gRPC server at {addr}. Stop it with ctrl + c");

                if let Err(err) = server.await.map_err(ServerError::Tonic) {
                    eprintln!("Error: {err}");
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::test]
async fn test_service() -> Result<(), ServerError> {
    let service = ValidatorService::new()?;
    for _ in 0..100 {
        // Chrome is unknown
        let ua = fakeit::user_agent::chrome();
        println!("chrome: {ua}");
        let result = service
            .user_agent(Request::new(ValidateUserAgentRequest {
                user_agent: ua.into(),
            }))
            .await?;

        assert_eq!(result.into_inner().validity, i32::from(Validity::Unknown));

        // Safari is invalid
        let ua = fakeit::user_agent::safari();
        println!("safari: {ua}");
        let result = service
            .user_agent(Request::new(ValidateUserAgentRequest {
                user_agent: ua.into(),
            }))
            .await?;

        assert_eq!(result.into_inner().validity, i32::from(Validity::Invalid));

        // Firefox is valid
        let ua = fakeit::user_agent::firefox();
        println!("firefox: {ua}");
        let result = service
            .user_agent(Request::new(ValidateUserAgentRequest {
                user_agent: ua.into(),
            }))
            .await?;

        assert_eq!(result.into_inner().validity, i32::from(Validity::Valid));

        // Opera is unknown
        let ua = fakeit::user_agent::opera();
        println!("opera: {ua}");
        let result = service
            .user_agent(Request::new(ValidateUserAgentRequest {
                user_agent: ua.into(),
            }))
            .await?;

        assert_eq!(result.into_inner().validity, i32::from(Validity::Unknown));
    }

    Ok(())
}
