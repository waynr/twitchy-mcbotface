use bevy::{
    ecs::query::QuerySingleError,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    sprite::Anchor,
    text::{BreakLineOn, Text2dBounds},
    window::WindowResolution,
    winit::WinitSettings,
};
use bevy_image_export::{ImageExportSource, NDIExport, NDIExportBundle, NDIExportPlugin};

pub mod chatbox;
pub use chatbox::ChatboxDispatcher;
pub use chatbox::ChatboxState;

use super::error::Result;

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
        .add_systems(Update, chat_text_bundle_update_system)
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    let box_position = Vec2::new(0.0, -250.0);

    let text_style = TextStyle {
        font_size: 42.0,
        color: Color::WHITE,
        ..default()
    };

    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.0, 0.25, 0.75),
                custom_size: Some(Vec2::new(box_size.x, box_size.y)),
                ..default()
            },
            transform: Transform::from_translation(box_position.extend(0.0)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((
                Text2dBundle {
                    text: Text {
                        sections: vec![TextSection::new("meow", text_style.clone())],
                        alignment: TextAlignment::Left,
                        linebreak_behavior: BreakLineOn::WordBoundary,
                    },
                    text_2d_bounds: Text2dBounds { size: box_size },
                    transform: Transform::from_translation(Vec3::new(
                        box_size.x / -2.0,
                        box_size.y / -2.0,
                        1.0,
                    )),
                    text_anchor: Anchor::BottomLeft,
                    ..default()
                },
                ChatTextBundle,
            ));
        });

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
struct ChatTextBundle;

fn chat_text_bundle_update_system(
    mut query: Query<&mut Text, With<ChatTextBundle>>,
    chatbox_state: Res<ChatboxState>,
) {
    let mut text = match query.get_single_mut() {
        Ok(text) => text,
        Err(QuerySingleError::NoEntities(_)) => {
            bevy::log::error!("no ChatTextBundle entity found");
            return;
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            bevy::log::error!("unexpectedly many ChatTextBundle entities found");
            return;
        }
    };
    let messages = chatbox_state
        .messages
        .lock()
        .expect("TODO: gracefully handle PoisonError");

    let username_style = TextStyle {
        font_size: 50.0,
        color: Color::RED,
        ..default()
    };
    let message_style = TextStyle {
        font_size: 42.0,
        ..default()
    };
    text.sections = messages
        .iter()
        .map(|m| {
            vec![
                TextSection::new("\n".to_string(), username_style.clone()),
                TextSection::new(m.user.clone(), username_style.clone()),
                TextSection::new(" ".to_string(), username_style.clone()),
                TextSection::new(m.message.clone(), message_style.clone()),
            ]
            .into_iter()
        })
        .flatten()
        .collect();
}
