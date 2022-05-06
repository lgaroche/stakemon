use log::{debug, info};
use std::fmt::{Display, Formatter};
use crate::balance::{ValidatorBalanceChecker, ValidatorBalanceError};


pub struct Monitor {
    db: sled::Db,
    balance_checker: ValidatorBalanceChecker
}

#[derive(Debug, Clone)]
pub struct Account {
    pub user_id: u64,
    pub validator_index: u64
}

#[derive(Debug, Clone)]
pub enum AlertMessage {
    NotRewarded { validator_index: u64 },
    Slashed { validator_index: u64, amount: u64 }
}

impl Display for AlertMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertMessage::NotRewarded { validator_index } =>
                write!(f, "Validator {} missed rewards", validator_index),
            AlertMessage::Slashed { validator_index, amount } =>
                write!(f, "Validator {} was slashed {} nano-mGNO", validator_index, amount)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub account: Account,
    pub alert: AlertMessage
}

#[derive(Debug)]
pub enum Error {
    SledError(sled::Error),
    BalanceError(ValidatorBalanceError)
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SledError(e) => write!(f, "{}", e),
            Error::BalanceError(e) =>write!(f, "{}", e)
        }
    }
}

impl std::error::Error for Error {}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::SledError(err)
    }
}

impl From<ValidatorBalanceError> for Error {
    fn from(err: ValidatorBalanceError) -> Self {
        Error::BalanceError(err)
    }
}

impl Account {
    pub fn new(user_id: u64, validator_index: u64) -> Self {
        Account { user_id, validator_index }
    }

    pub fn key(&self) -> Vec<u8> {
        let mut id = self.user_id.to_le_bytes().to_vec();
        id.append(self.validator_index.to_le_bytes().to_vec().as_mut());
        id
    }
}

impl From<Vec<u8>> for Account {
    fn from(key: Vec<u8>) -> Self {

        let mut id= [0u8; 8];
        id.copy_from_slice(&key[..8]);

        let mut index = [0u8; 8];
        index.copy_from_slice(&key[8..]);

        Account {
            user_id: u64::from_le_bytes(id),
            validator_index: u64::from_le_bytes(index)
        }
    }
}

impl Monitor {

    pub fn new(state_path: &str, validator_api_url: &str) -> Result<Self, Error> {
        Ok(Monitor {
            db: sled::open(state_path)?,
            balance_checker: ValidatorBalanceChecker::new(validator_api_url)
        })
    }

    pub fn watch(&self, account: Account) -> Result<(), Error> {
        match self.db.insert(account.key(), 0u64.to_le_bytes().as_slice()) {
            Err(e) => Err(Error::SledError(e)),
            Ok(_) => Ok(())
        }
    }

    pub fn _forget(&self, account: Account) -> Result<(), Error>{
        match self.db.remove(account.key()) {
            Err(e) => Err(Error::SledError(e)),
            Ok(_) => Ok(())
        }
    }

    pub async fn run(&self) -> Result<Vec<Alert>, Error> {

        info!("monitor run start: {} accounts monitored", self.db.len());
        let mut alerts = vec![];

        let accounts: Vec<(Account, u64)> = self.db
            .iter()
            .filter_map(|kv| kv.ok())
            .map(|(k, v) | {
                let mut prev_balance_bytes = [0u8; 8];
                prev_balance_bytes.copy_from_slice(v.to_vec().as_slice());
                (Account::from(k.to_vec()), u64::from_le_bytes(prev_balance_bytes))
            })
            .collect();

        let balances = self.balance_checker
            .get_balances(
                accounts.iter().map(|(a, _) | a.validator_index).collect()
            )
            .await?;

        for (account, prev_balance) in accounts {
            let key = account.key();
            let validator_index = account.validator_index;

            if let Some(b) = balances.get(validator_index.to_string().as_str()) {
                let new_balance = b.parse::<u64>().unwrap_or(0);

                debug!("balance for {:?} balance diff: {}", account, new_balance as i64 - prev_balance as i64);

                if new_balance == prev_balance {
                    info!("account was not rewarded: {:?}", account);
                    alerts.push(Alert {
                        account,
                        alert: AlertMessage::NotRewarded { validator_index }
                    })
                } else if new_balance < prev_balance {
                    let amount = prev_balance - new_balance;
                    info!("account was slashed {} units: {:?}", amount, account);
                    alerts.push(Alert {
                        account,
                        alert: AlertMessage::Slashed { validator_index, amount }
                    })
                }
                self.db.insert(key, new_balance.to_le_bytes().as_slice())?;
            } else {
                log::warn!("balance not found for validator {}", account.validator_index)
            }
        };

        info!("{} alerts to send", alerts.len());
        Ok(alerts)
    }
}