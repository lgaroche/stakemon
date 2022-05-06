use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::StandardFramework;
use tokio::time;
use tokio::task;
use log::{info, error, warn};
use serenity::model::prelude::application_command::{ApplicationCommand, ApplicationCommandInteractionDataOption, ApplicationCommandOptionType};
use crate::monitor::{Monitor, Account};

struct Handler;

struct MonitorData;
impl TypeMapKey for MonitorData {
    type Value = Monitor;
}

pub struct Bot {
    monitor: Monitor,
    token: String
}

impl Bot {

    pub fn new(token: String, monitor: Monitor) -> Self {
        Self { monitor, token }
    }

    pub async fn start(self) -> serenity::Result<()> {
        let framework = StandardFramework::new()
            .configure(|c| c.prefix("/"));
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;
        let mut client =
            Client::builder(self.token, intents)
                .event_handler(Handler)
                .framework(framework)
                .await?;

        client.data
            .write().await
            .insert::<MonitorData>(self.monitor);

        client.start().await
    }
}



#[async_trait]
impl EventHandler for Handler {

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        if let Err(why) = ApplicationCommand::set_global_application_commands(&ctx.http, |c| {
            c.create_application_command(|command| {
                command
                    .name("watch")
                    .description("Starts monitoring an account")
                    .create_option(|opt| {
                        opt
                            .name("account")
                            .description("Validator index of the account to monitor")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
            }).create_application_command(|command| {
                command
                    .name("forget")
                    .description("Stops monitoring an account")
                    .create_option(|opt| {
                        opt
                            .name("account")
                            .description("Validator index of the account to forget")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
            })
        }).await {
            panic!("failed to create application commands: {}", why)
        }

        task::spawn(async move {
            let data = ctx.data.read().await;
            if let Some(monitor) = data.get::<MonitorData>() {

                let mut interval = time::interval(time::Duration::from_secs(300));
                loop {
                    interval.tick().await;

                    let alerts = match monitor.run().await {
                        Ok(a) => a,
                        Err(e) => {
                            error!("failed to get alerts: {}", e);
                            vec![]
                        }
                    };

                    for alert in alerts.iter() {
                        let user_id = UserId(alert.account.user_id);
                        if let Ok(channel) = UserId::create_dm_channel(user_id, &ctx.http).await {
                            if let Err(e) = channel.say(&ctx.http, format!("{}", alert.alert)).await {
                                error!("failed to send message: {}", e);
                            }
                        };
                    }
                }
            }
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "watch" => {
                    let data = ctx.data.read().await;
                    match data.get::<MonitorData>() {
                        Some(monitor) => {
                            match unwrap_first_option_as_u64(&command.data.options) {
                                Some(validator_index) => {
                                    let account = Account::new(command.user.id.0, validator_index);
                                    match monitor.watch(account) {
                                        Ok(()) => {
                                            info!("start watching {}", validator_index);
                                            format!("Started watching validator {}, \
                                                        I will check your validator balance \
                                                        every 5 minutes and send you a direct \
                                                        message if something goes wrong", validator_index).to_string()
                                        },
                                        Err(e) => {
                                            error!("failed to start watching: {}", e);
                                            "failed to start watching".to_string()
                                        }
                                    }
                                }
                                None => "wrong argument".to_string()
                            }
                        }
                        None => "failed to start watching".to_string()
                    }
                }
                "forget" => {
                    let data = ctx.data.read().await;
                    match data.get::<MonitorData>() {
                        Some(monitor) => {
                            match unwrap_first_option_as_u64(&command.data.options) {
                                Some(validator_index) => {
                                    let account = Account::new(command.user.id.0, validator_index);
                                    match monitor.forget(account) {
                                        Ok(()) => {
                                            info!("forget {}", validator_index);
                                            format!("Stopped watching validator {}", validator_index).to_string()
                                        },
                                        Err(e) => {
                                            error!("failed to stop watching: {}", e);
                                            "failed to start watching".to_string()
                                        }
                                    }
                                }
                                None => "wrong argument".to_string()
                            }
                        }
                        None => "failed to start watching".to_string()
                    }
                }
                _ => "not implemented".to_string()
            };

            if let Err(why) = command.create_interaction_response(&ctx.http, |res| {
                res
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content(content))
            }).await {
                warn!("Cannot respond to command: {}", why);
            }
        }
    }
}

fn unwrap_first_option_as_u64(options: &Vec<ApplicationCommandInteractionDataOption>) -> Option<u64> {
    match options.first().as_ref() {
        Some(&option) => match &option.value {
            Some(v) => v.as_u64(),
            None => None
        },
        None => None
    }
}