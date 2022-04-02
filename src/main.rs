use std::fs::File;
use std::io::Read;
use std::thread;

use futures::future::join3;
use tokio::sync::oneshot;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;

use tmbf::commander::{CommanderComposer, HardCodedCommander};
use tmbf::error::{Error, Result};
use tmbf::irc::{ComponentMessage, IrcCore, JoinChannelMessage, MessageDispatcher};
use tmbf::egui_ui::glutin_event_loop;

fn main() -> Result<()> {
    let (sender, receiver) = oneshot::channel::<MessageDispatcher>();
    thread::spawn(move || {
        if let Err(error) = all_the_async_things(sender) {
            println!("all (or some) of the async things failed: {}", error);
        }
    });

    glutin_event_loop(receiver)
}

#[tokio::main]
pub async fn all_the_async_things(sender: oneshot::Sender<MessageDispatcher>) -> Result<()> {
    let mut file = File::open("/home/wayne/.config/twitchy-mcbotface/auth.yml")?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;
    let login_creds: StaticLoginCredentials = serde_yaml::from_str(&contents)?;

    let config = ClientConfig::new_simple(login_creds);
    let mut core = IrcCore::new();
    let join_dispatcher = core.get_msg_dispatcher();

    if let Err(_error) = sender.send(join_dispatcher.clone()) {
        return Err(Error::SomethingBad(String::from(
            "Receiver<MessageDispatcher> dropped",
        )));
    }

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

    let (_, run_irc_result, joiner_result) =
        join3(cmdr_handle, run_irc_handle, joiner_handler).await;
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
    Ok(())
}
