# Stakemon
Discord bot that monitors your beacon chain validators. 
It checks every 5 minutes the validator balances, and sends a direct message to the user
if the validator was not rewarded or if their stake was slashed.   

## Usage
Once the bot has been joined your server, the following commands are available in the chat:
- ```/watch {validator_index}``` Will start watching this validator
- ```/forget {validator_index``` Stops watching this validator

```validator_index``` is the numeric index of the validator, it can be found 
in Prysm interface or on the beacon chain explorer.

## Installation
### Build
```console
cargo build --release
```
### Environment variables
- ```NODE_API_URL``` The REST endpoint of a beacon node used to get the balances (by default on port 3500)
- ```DISCORD_TOKEN``` The Discord Build-A-Bot token