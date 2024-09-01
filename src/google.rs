use crate::api::GoogleTrendApi;
use crate::api::API;
use crate::client::Client;
use crate::error::Result;
use crate::utils::build_request;
use std::collections::BTreeMap;

pub struct GoogleTrend {
    pub client: Client,
}

impl GoogleTrend {
    pub fn keywords(&self) -> Result<Vec<String>> {
        let keywords: Vec<String> = self
            .client
            .get(API::GoogleTrendApi(GoogleTrendApi::Keywords), None)?;

        Ok(keywords)
    }

    pub fn get<K>(&self, keyword: K) -> Result<Vec<Vec<String>>>
    where
        K: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("keyword".into(), keyword.into());
        let request = build_request(parameters);
        let forex_data: Vec<Vec<String>> = self
            .client
            .get(API::GoogleTrendApi(GoogleTrendApi::Trend), Some(request))?;

        Ok(forex_data)
    }
}
