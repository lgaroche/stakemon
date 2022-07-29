mod bot;
mod monitor;
mod balance;

use std::error::Error;
use std::env;
use crate::bot::{Bot};
use crate::monitor::{Monitor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{

    tracing_subscriber::fmt::init();

    let validator_api_url = env::var("NODE_API_URL").expect("missing NODE_API_URL");
    let token = env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let db_path = env::var("DB_PATH").unwrap_or("./state".to_string());
    let interval_str = env::var("MONITOR_INTERVAL").unwrap_or("300".to_string());
    let interval = interval_str.parse::<u64>().expect("failed to parse MONITOR_INTERVAL");

    let monitor = Monitor::new(db_path.as_str(), validator_api_url.as_str(), interval)?;

    Bot::new(token, monitor).start().await?;

    Ok(())
}
