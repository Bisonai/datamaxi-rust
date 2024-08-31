use crate::api::API;
use crate::error::Result;
use crate::error::{self};
use reqwest::blocking::Response;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::io::Read;

const HOST: &str = "https://api.datamaxiplus.com/api/v1";

#[derive(Clone)]
pub struct Client {
    host: String,
    api_key: String,
    inner_client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Client {
            host: HOST.to_string(),
            api_key,
            inner_client: reqwest::blocking::Client::builder()
                .pool_idle_timeout(None)
                .build()
                .unwrap(),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, endpoint: API, request: Option<String>) -> Result<T> {
        let mut url: String = format!("{}{}", self.host, String::from(endpoint));
        if let Some(request) = request {
            if !request.is_empty() {
                url.push_str(format!("?{}", request).as_str());
            }
        }

        let client = &self.inner_client;
        let response = client
            .get(url.as_str())
            .header("X-DTMX-APIKEY", &self.api_key)
            .send()?;

        self.handle_response(response)
    }

    fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        match response.status() {
            StatusCode::OK => Ok(response.json::<T>()?),
            StatusCode::INTERNAL_SERVER_ERROR => {
                let mut response_text = String::new();
                response.take(1000).read_to_string(&mut response_text)?;
                Err(error::ErrorKind::InternalServerError(response_text).into())
            }
            StatusCode::SERVICE_UNAVAILABLE => Err(error::ErrorKind::ServiceUnavailable.into()),
            StatusCode::UNAUTHORIZED => Err(error::ErrorKind::Unauthorized.into()),
            StatusCode::BAD_REQUEST => {
                let mut response_text = String::new();
                response.take(1000).read_to_string(&mut response_text)?;
                Err(error::ErrorKind::BadRequest(response_text).into())
            }
            status => Err(error::ErrorKind::UnexpectedStatusCode(status.as_u16()).into()),
        }
    }
}
