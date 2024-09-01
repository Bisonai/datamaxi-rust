use crate::api::ForexApi;
use crate::api::API;
use crate::client::Client;
use crate::error::Result;
use crate::models::ForexDetail;

pub struct Forex {
    pub client: Client,
}

impl Forex {
    pub fn symbols(&self) -> Result<Vec<String>> {
        let symbols: Vec<String> = self.client.get(API::ForexApi(ForexApi::Symbols), None)?;

        Ok(symbols)
    }

    pub fn get(&self) -> Result<Vec<ForexDetail>> {
        let forex_data: Vec<ForexDetail> = self.client.get(API::ForexApi(ForexApi::Forex), None)?;

        Ok(forex_data)
    }
}
