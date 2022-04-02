use glow::{HasContext, PixelPackData};
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
            create_ndi_send_video_frame(data.width, data.height, FrameFormatType::Progressive)
                .with_data(data.buf, data.width * 4, SendColorFormat::Rgba);

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

pub struct NDIFrameData {
    pub buf: Vec<u8>,
    width: i32,
    height: i32,
}

impl TryFrom<(i32, i32)> for NDIFrameData {
    type Error = Error;

    fn try_from(t: (i32, i32)) -> Result<Self> {
        let capacity: usize = TryInto::<usize>::try_into(t.0 * t.1)? * 4;

        let mut buf: Vec<u8> = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            // vec must actually be populated before it can be written to by the opengl bindings.
            // use a semitransparent grey image to help debugging in case something else goes wrong
            // when grabbing the image from the gpu
            buf.push(244)
        }

        Ok(Self {
            buf,
            width: t.0,
            height: t.1,
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
            gl.read_pixels(0, 0, self.width, self.height, 0x1908, 0x1401, pixels);
        }
    }
}
