use std::sync::Arc;
use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    window::WindowResolution,
    winit::WinitSettings,
};
use bevy_image_export::{ImageExportSource, NDIExport, NDIExportBundle, NDIExportPlugin};

pub mod chatbox;
use chatbox::Chatbox;
pub use chatbox::ChatboxDispatcher;
pub use chatbox::ChatboxState;

use super::error::Result;

pub enum BotfaceEvent {
    Nonce,
}

pub struct Botface {
    chatbox: Chatbox,
    app: App,
}

impl Botface {
    pub fn new() -> Result<Self> {
        let chatbox_state = Arc::new(Mutex::new(ChatboxState::new()));
        let chatbox = Chatbox::new(chatbox_state);

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
            bevy::diagnostic::LogDiagnosticsPlugin { ..default() },
        ))
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup);

        Ok(Self { chatbox, app })
    }

    pub fn chatbox_state(&self) -> Arc<Mutex<ChatboxState>> {
        self.chatbox.state()
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
            width: 768,
            height: 768,
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
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(4.2 * Vec3::Z),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Camera3dBundle {
                camera: Camera {
                    target: RenderTarget::Image(output_texture_handle.clone()),
                    ..default()
                },
                ..default()
            });
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
