use std::fs::File;
use std::io::Read;

use futures::future::join4;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;

use router::commander::{CommanderComposer, HardCodedCommander};
use router::error::Result;
use router::irc::{ComponentMessage, IrcCore, JoinChannelMessage};
use router::router::TwitchRouter;

#[tokio::main]
async fn main() -> Result<()> {
    let mut file = File::open("/home/wayne/.config/twitchy-mcbotface/auth.yml")?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;
    let login_creds: StaticLoginCredentials = serde_yaml::from_str(&contents)?;

    let config = ClientConfig::new_simple(login_creds);
    let mut core = IrcCore::new();
    let join_dispatcher = core.get_msg_dispatcher();

    let router_dispatcher = core.get_msg_dispatcher();
    let router = TwitchRouter::new(router_dispatcher);
    let router_handler = tokio::spawn(async move {
        match router.run().await {
          Ok(_) => (),
          Err(e) => eprintln!("twitch router failed: {e}"),
        };
    });

    let run_irc_handle = core.run_irc(config);

    let cmdr_dispatcher = join_dispatcher.clone();
    let hard_coded_cmdr = Box::new(HardCodedCommander::new("TODO".to_string()));
    let mut cmdr_composer = CommanderComposer::new(cmdr_dispatcher, vec![hard_coded_cmdr]);
    let cmdr_handle = cmdr_composer.run_commanders();

    let joiner_handler = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        match join_dispatcher
            .sender
            .send(ComponentMessage::JoinChannel(JoinChannelMessage {
                channel: "uuayn".to_string(),
            })) {
            Err(e) => {
                println!("failed to join uuayn channel: {}", e)
            }
            _ => (),
        }
    });

    let (_, run_irc_result, joiner_result, router_result) =
        join4(cmdr_handle, run_irc_handle, joiner_handler, router_handler).await;
    match joiner_result {
        Err(e) => {
            println!("joiner failed: {}", e)
        }
        _ => (),
    };
    match run_irc_result {
        Err(e) => {
            println!("run_irc failed: {}", e)
        }
        _ => (),
    };
    match router_result {
        Err(e) => {
            println!("router failed: {}", e)
        }
        _ => (),
    };
    Ok(())
}
