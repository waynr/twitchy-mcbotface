use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use futures::future::join5;
use tokio::sync::mpsc;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;

use glutin::event_loop::EventLoopProxy;

use tmbf::commander::{CommanderComposer, HardCodedCommander};
use tmbf::egui_ui::{Botface, BotfaceEvent, ChatboxDispatcher, ChatboxState};
use tmbf::error::Result;
use tmbf::irc::{ComponentMessage, IrcCore, JoinChannelMessage, MessageDispatcher};
use tmbf::ndi::{NDIFrameData, NDIPainter};

fn main() -> Result<()> {
    let (frame_sender, frame_receiver) = mpsc::unbounded_channel::<NDIFrameData>();
    let botface = Botface::new(frame_sender)?;
    let chatbox_state = botface.chatbox_state();
    let event_loop_proxy = botface.event_loop_proxy();
    thread::spawn(move || {
        if let Err(error) = all_the_async_things(frame_receiver, event_loop_proxy, chatbox_state) {
            println!("all (or some) of the async things failed: {}", error);
        }
    });

    botface.run_event_loop()
}

#[tokio::main]
pub async fn all_the_async_things(
    frame_receiver: mpsc::UnboundedReceiver<NDIFrameData>,
    event_loop_proxy: EventLoopProxy<BotfaceEvent>,
    chatbox_state: Arc<Mutex<ChatboxState>>,
) -> Result<()> {
    let mut file = File::open("/home/wayne/.config/twitchy-mcbotface/auth.yml")?;
    let mut contents = String::new();

    let mut ndi_painter = NDIPainter::new()?;
    let ndi_painter_handle = ndi_painter.run(frame_receiver);

    file.read_to_string(&mut contents)?;
    let login_creds: StaticLoginCredentials = serde_yaml::from_str(&contents)?;

    let config = ClientConfig::new_simple(login_creds);
    let mut core = IrcCore::new();
    let join_dispatcher = core.get_msg_dispatcher();

    let mut chatbox_dispatcher =
        ChatboxDispatcher::new(join_dispatcher.clone(), chatbox_state, event_loop_proxy);
    let chatbox_dispatcher_handle = chatbox_dispatcher.run();

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

    let (_, run_irc_result, joiner_result, _, _) = join5(
        cmdr_handle,
        run_irc_handle,
        joiner_handler,
        ndi_painter_handle,
        chatbox_dispatcher_handle,
    )
    .await;
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
