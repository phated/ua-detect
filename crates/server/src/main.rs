use tonic::{transport::Server, Request, Response, Status};

use validate::validate_server::{Validate, ValidateServer};
use validate::{ValidateUserAgentRequest, ValidateUserAgentResponse};

pub mod validate {
    tonic::include_proto!("validate");
}

#[derive(Debug, Default)]
pub struct ValidatorService;

#[tonic::async_trait]
impl Validate for ValidatorService {
    async fn validate_user_agent(
        &self,
        request: Request<ValidateUserAgentRequest>,
    ) -> Result<Response<ValidateUserAgentResponse>, Status> {
        println!("Got a request: {:?}", request);

        let reply = ValidateUserAgentResponse { is_valid: false };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:9001".parse()?;
    let validator = ValidatorService::default();

    Server::builder()
        .add_service(ValidateServer::new(validator))
        .serve(addr)
        .await?;

    Ok(())
}
