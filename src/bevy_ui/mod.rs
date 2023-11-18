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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        .spawn(Camera2dBundle {
            transform: Transform {
                translation: Vec3::Z * 4.0,
                ..default()
            },
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

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));
    let default_material = StandardMaterial {
        base_color: Color::rgb(0.2, 0.7, 0.6),
        reflectance: 0.2,
        unlit: false,
        ..default()
    };
    let preview_material_handle = materials.add(default_material.clone());

    // The cube that will be rendered to the texture.
    commands.spawn(PbrBundle {
        mesh: cube_handle,
        material: preview_material_handle,
        transform: Transform {
            rotation: Quat::from_xyzw(0.4, 0.2, 0.2, 1.0),
            ..default()
        },
        ..default()
    });

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
        ..default()
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
