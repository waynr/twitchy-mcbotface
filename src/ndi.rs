use egui::State;
use glow::{HasContext, PixelPackData};
use winit::dpi::PhysicalSize;
use ndi_sdk::send::{create_ndi_send_video_frame, FrameFormatType, SendColorFormat};
use ndi_sdk::{load, SendInstance};
use tokio::sync::mpsc;

use crate::error::{Error, Result};

pub struct NDIPainter {
    sender: SendInstance,
}

impl NDIPainter {
    pub fn new() -> Result<Self> {
        // set up NDI SDK for sending
        let instance = match load() {
            Ok(ndi) => ndi,
            Err(s) => return Err(Error::NDISDKError(s)),
        };
        let sender = instance.create_send_instance("chatbox".to_string(), false, false)?;
        Ok(Self { sender })
    }

    pub fn paint(&mut self, data: NDIFrameData) -> Result<()> {
        let frame_builder =
            create_ndi_send_video_frame(data.size.x, data.size.y, FrameFormatType::Progressive)
                .with_data(data.buf, data.size.x * 4, SendColorFormat::Rgba);

        let frame = frame_builder.build()?;

        self.sender.send_video(frame);
        Ok(())
    }

    pub async fn run(&mut self, mut receiver: mpsc::UnboundedReceiver<NDIFrameData>) {
        while let Some(frame_data) = receiver.recv().await {
            match self.paint(frame_data) {
                Err(e) => println!("{}", e),
                _ => (),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pos2 {
    /// How far to the right.
    pub x: i32,
    /// How far down.
    pub y: i32,
}

impl From<emath::Pos2> for Pos2 {
    fn from(pos: emath::Pos2) -> Self {
        Self { x: pos.x.floor() as i32, y: pos.y.floor() as i32 }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    /// Rightwards. Width.
    pub x: i32,
    /// Downwards. Height.
    pub y: i32,
}

impl From<emath::Vec2> for Vec2 {
    fn from(vec: emath::Vec2) -> Self {
        Self { x: vec.x.floor() as i32, y: vec.y.floor() as i32 }
    }
}

impl From<PhysicalSize<u32>> for Vec2 {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self { x: size.width as i32, y: size.height as i32 }
    }
}

pub struct NDIFrameData {
    pub buf: Vec<u8>,

    position: Pos2,
    size: Vec2,
    outer_window_size: Vec2,
}

impl TryFrom<(State, PhysicalSize<u32>)> for NDIFrameData {
    type Error = Error;

    fn try_from(data: (State, PhysicalSize<u32>)) -> Result<Self> {
        let size: Vec2 = data.0.size.into();
        let position: Pos2 = data.0.pos.into();
        let outer_window_size: Vec2 = data.1.into();
        let capacity: usize = TryInto::<usize>::try_into(size.x * size.y)? * 4;

        let mut buf: Vec<u8> = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            // vec must actually be populated before it can be written to by the opengl bindings.
            // use a semitransparent grey image to help debugging in case something else goes wrong
            // when grabbing the image from the gpu
            buf.push(244)
        }

        Ok(Self {
            buf,
            position,
            size,
            outer_window_size,
        })
    }
}

impl NDIFrameData {
    pub fn get_pixels(&mut self, gl: &glow::Context) {
        let pixels = PixelPackData::Slice(&mut self.buf);
        unsafe {
            // from https://docs.gl/gl4/glReadPixels,
            // format = 0x1908 should match to GL_RGBA
            // gltype = 0x1401 should match to GL_UNSIGNED_BYTE
            gl.read_pixels(
                self.position.x,
                self.outer_window_size.y - (self.position.y + self.size.y),
                self.size.x,
                self.size.y,
                0x1908,
                0x1401,
                pixels,
            );
        }
    }
}
