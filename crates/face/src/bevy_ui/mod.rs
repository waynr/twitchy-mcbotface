use std::time::{Duration, Instant};

use bevy::{
    ecs::query::QuerySingleError,
    ecs::system::EntityCommands,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    text::BreakLineOn,
    window::WindowResolution,
    winit::WinitSettings,
};
use bevy_image_export::{ImageExportSource, NDIExport, NDIExportBundle, NDIExportPlugin};

pub mod chatbox;
pub use chatbox::ChatboxDispatcher;
pub use chatbox::ChatboxState;

use super::error::Result;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Message(String);

#[derive(Component)]
struct MessageReceivedTime(Instant);

pub enum BotfaceEvent {
    Nonce,
}

pub struct Botface {
    chatbox_state: ChatboxState,
    app: App,
}

impl Botface {
    pub fn new() -> Result<Self> {
        let chatbox_state = ChatboxState::default();

        let mut app = App::new();
        app.insert_resource(WinitSettings {
            return_from_run: true,
            ..default()
        })
        .insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(1.0/60.0)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(768.0, 768.0).with_scale_factor_override(1.0),
                    transparent: true,
                    present_mode: bevy::window::PresentMode::Fifo,
                    ..default()
                }),
                ..default()
            }),
            NDIExportPlugin,
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            //bevy::diagnostic::LogDiagnosticsPlugin { ..default() },
        ))
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(chatbox_state.clone())
        .init_resource::<Events<ApplyChatMessagesGradient>>()
        .add_systems(FixedUpdate, new_chat_message_event)
        .add_systems(
            FixedUpdate,
            apply_chat_messages_gradient.after(new_chat_message_event),
        )
        .add_systems(Startup, setup);

        Ok(Self { chatbox_state, app })
    }

    pub fn chatbox_state(&self) -> ChatboxState {
        self.chatbox_state.clone()
    }

    pub fn run(mut self) {
        self.app.run();
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
) {
    let output_texture_handle = {
        let size = Extent3d {
            width: 1920,
            height: 1080,
            ..default()
        };
        let mut export_texture = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        export_texture.resize(size);

        images.add(export_texture)
    };

    commands
        .spawn(Camera2dBundle {
            transform: Transform { ..default() },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Image(output_texture_handle.clone()),
                    ..default()
                },
                ..default()
            });
        });

    let box_size = Vec2::new(1000.0, 500.0);
    //let box_position = Vec2::new(0.0, -250.0);

    let nb = (
        NodeBundle {
            style: Style {
                width: Val::Px(box_size.x),
                height: Val::Px(box_size.y),
                justify_content: JustifyContent::FlexEnd,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        },
        ChatNodeBundle,
    );
    commands.spawn(nb);

    match NDIExport::new("chatbox".to_string()) {
        Err(e) => eprintln!("failed to initialize NDIExport: {e}"),
        Ok(ndi_export) => {
            commands.spawn(NDIExportBundle {
                source: export_sources.add(output_texture_handle.into()),
                export: ndi_export,
            });
        }
    }
}

#[derive(Component)]
struct ChatNodeBundle;

fn new_chat_message_event(
    mut commands: Commands,
    chatbox_state: Res<ChatboxState>,
    user_query: Query<(Entity, &Name)>,
    mut chat_node_bundle: Query<(Entity, With<ChatNodeBundle>)>,
    mut events: ResMut<Events<ApplyChatMessagesGradient>>,
) {
    let mut incoming = chatbox_state.incoming.lock().expect("TODO");
    if incoming.len() == 0 {
        return;
    }

    let node_bundle_id = match chat_node_bundle.get_single_mut() {
        Ok(id) => id.0,
        Err(QuerySingleError::NoEntities(_)) => {
            bevy::log::error!("no ChatNodeBundle entity found");
            return;
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            bevy::log::error!("unexpectedly many ChatNodeBundle entities found");
            return;
        }
    };

    let mut messages = chatbox_state.messages.lock().expect("TODO");

    for message in incoming.drain(..) {
        let user_id = if let Some(entity) = user_query.iter().find(|(_, n)| n.0 == message.user) {
            commands.get_entity(entity.0).unwrap().id()
        } else {
            commands.spawn(Name(message.user.clone())).id()
        };
        commands
            .spawn(Message(message.message.clone()))
            .set_parent(user_id);

        let mut nb_commands = if let Some(nb_commands) = commands.get_entity(node_bundle_id) {
            nb_commands
        } else {
            return; // no entity found
        };

        update_node_bundle(
            message.message.as_str(),
            message.user.as_str(),
            &mut nb_commands,
        );
        messages.push(message);
    }

    events.send(ApplyChatMessagesGradient);
}

fn update_node_bundle(message: &str, user: &str, node_bundle_ec: &mut EntityCommands) {
    let username_style = &TextStyle {
        font_size: 50.0,
        color: Color::RED.with_a(TRANSPARENCY_CHAT_FG_MAX),
        ..default()
    };
    let message_style = TextStyle {
        font_size: 42.0,
        color: Color::WHITE.with_a(TRANSPARENCY_CHAT_FG_MAX),
        ..default()
    };

    node_bundle_ec.with_children(|child_builder| {
        let text_bundle = TextBundle {
            text: Text {
                sections: vec![
                    TextSection::new(format!("{user} "), username_style.clone()),
                    TextSection::new(format!("{message} "), message_style.clone()),
                ],
                alignment: TextAlignment::Left,
                linebreak_behavior: BreakLineOn::WordBoundary,
            },
            transform: Transform::from_translation(Vec3::Z),
            background_color: Color::rgba(0.0, 0.25, 0.75, TRANSPARENCY_CHAT_BG_MAX).into(),
            ..default()
        };
        child_builder.spawn((text_bundle, MessageReceivedTime(Instant::now())));
    });
}

#[derive(Event)]
struct ApplyChatMessagesGradient;

const TRANSPARENCY_SETTLE_TIME: u64 = 15;
const TRANSPARENCY_MIN: f32 = 0.25;
const TRANSPARENCY_CHAT_BG_MAX: f32 = 0.6;
const TRANSPARENCY_CHAT_FG_MAX: f32 = 0.9;

fn apply_chat_messages_gradient(
    chat_text_children: Query<(&Children, With<ChatNodeBundle>)>,
    mut q_text: Query<(
        &mut BackgroundColor,
        &mut Text,
        &MessageReceivedTime,
        With<Parent>,
    )>,
    mut events: ResMut<Events<ApplyChatMessagesGradient>>,
) {
    let mut continue_applying_gradients = false;
    let _ = events.drain();

    let settle_time = Duration::from_secs(TRANSPARENCY_SETTLE_TIME);
    for (children, _) in chat_text_children.iter() {
        for &child in children.iter() {
            let (mut bg_color, mut text, message_received_time, _) = q_text
                .get_mut(child)
                .expect("this text should always exist!");
            let since_received = message_received_time.0.elapsed();
            if since_received < settle_time {
                continue_applying_gradients = true;
            } else {
                continue;
            }

            let scale = since_received.as_secs_f32() / settle_time.as_secs_f32();
            let bg_transparency =
                TRANSPARENCY_CHAT_BG_MAX - (TRANSPARENCY_CHAT_BG_MAX - TRANSPARENCY_MIN) * scale;
            let fg_transparency =
                TRANSPARENCY_CHAT_FG_MAX - (TRANSPARENCY_CHAT_FG_MAX - TRANSPARENCY_MIN) * scale;

            bg_color.0.set_a(bg_transparency);
            for section in &mut text.sections {
                section.style.color.set_a(fg_transparency);
            }
        }
    }

    if continue_applying_gradients {
        events.send(ApplyChatMessagesGradient);
    }
}
