use thiserror::Error;
use tonic::{transport::Server, Request, Response, Status};

use uaparser::{Parser, UserAgentParser};
use validate::validate_server::{Validate, ValidateServer};
use validate::{ValidateUserAgentRequest, ValidateUserAgentResponse, Validity};

mod validate {
    tonic::include_proto!("validate");
}

#[derive(Debug, Error)]
enum ServerError {
    #[error("Could not construct UserAgentParser: {0}")]
    UserAgentParser(uaparser::Error),

    #[error("An error occurred with gRPC: {0}")]
    GrpcStatus(#[from] Status),
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
        let ua_parser: uaparser::UserAgentParser =
            uaparser::UserAgentParser::from_bytes(include_bytes!("regexes.yaml"))?;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:9001".parse()?;
    let validator = ValidatorService::new().unwrap();

    Server::builder()
        .add_service(ValidateServer::new(validator))
        .serve(addr)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_service() -> Result<(), ServerError> {
    let service = ValidatorService::new()?;
    for _ in 1..100 {
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
