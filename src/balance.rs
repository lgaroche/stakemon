use crate::balance::ValidatorBalanceError::{ReqwestError};
use std::fmt::{Debug, Formatter, Display};
use serde::Deserialize;
use std::collections::HashMap;
use log::{debug};
use std::cmp::min;

pub struct ValidatorBalanceChecker {
    client: reqwest::Client,
    url: String
}

#[derive(Debug, Deserialize)]
struct ValidatorBalance {
    index: String,
    balance: String
}

#[derive(Debug, Deserialize)]
struct GetBalanceResponse {
    data: Vec<ValidatorBalance>
}

#[derive(Debug)]
pub enum ValidatorBalanceError {
    ReqwestError(reqwest::Error),
}

impl Display for ValidatorBalanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReqwestError(e) => write!(f, "{}", e)
        }
    }
}

impl std::error::Error for ValidatorBalanceError {}

impl From<reqwest::Error> for ValidatorBalanceError {
    fn from(err: reqwest::Error) -> Self {
        ReqwestError(err)
    }
}

impl ValidatorBalanceChecker {

    pub fn new(api_url: &str) -> Self {
        let mut url = api_url.to_string();
        url.push_str("/eth/v1/beacon/states/head/validator_balances");
        ValidatorBalanceChecker {
            client: reqwest::Client::new(),
            url
        }
    }

    pub async fn get_balances(&self, validator_indexes: Vec<u64>) -> Result<HashMap<String, String>, ValidatorBalanceError> {

        let batch_size = 512;
        let mut n = 0;
        let len = validator_indexes.len();

        let mut balances = HashMap::<String, String>::new();

        while n < len {
            let end = min(n + batch_size, len);

            let v = validator_indexes[n..end]
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

            let mut url = self.url.clone();
            url.push_str("?id=");
            url.push_str(v.as_str());

            debug!("batch url: {}", url);

            let res = self.client.get(url).send().await?;
            let json = res.json::<GetBalanceResponse>().await?;

            balances.extend(json.data.iter().map(|b| {
                (b.index.clone(), b.balance.clone())
            }));

            n = n + batch_size;
        }

        Ok(balances)
    }
}