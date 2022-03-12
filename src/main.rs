mod error;
use glow::{HasContext, PixelPackData};

use crate::error::{Error, Result};

fn create_display(
    event_loop: &glutin::event_loop::EventLoop<()>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("twitchy mcbotface");

    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_srgb(true)
            .with_stencil_buffer(0)
            .with_vsync(true)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    unsafe {
        use glow::HasContext as _;
        gl.enable(glow::FRAMEBUFFER_SRGB);
    }

    (gl_window, gl)
}

fn main() -> Result<()> {
    // set up NDI SDK for sending
    let ndi = match ndi_sdk::load() {
        Ok(ndi) => ndi,
        Err(s) => return Err(Error::NDISDKError(s)),
    };

    let mut sender = ndi.create_send_instance("chatbox".to_string(), false, false)?;

    // egui/glow stuff
    let mut clear_color = [0.1, 0.1, 0.1];

    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let (gl_window, gl) = create_display(&event_loop);

    let mut egui_glow = egui_glow::EguiGlow::new(gl_window.window(), &gl);

    event_loop.run(move |event, _, control_flow: &mut glutin::event_loop::ControlFlow | {
        let mut redraw = || {
            let mut quit = false;

            let needs_repaint = egui_glow.run(gl_window.window(), |egui_ctx| {
                egui::SidePanel::left("my_side_panel").show(egui_ctx, |ui| {
                    ui.heading("Hello World!");
                    if ui.button("Quit").clicked() {
                        quit = true;
                    }
                    ui.color_edit_button_rgb(&mut clear_color);
                });
            });

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                gl_window.window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                unsafe {
                    use glow::HasContext as _;
                    gl.clear_color(clear_color[0], clear_color[1], clear_color[2], 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                // draw things behind egui here

                egui_glow.paint(gl_window.window(), &gl);

                let window_size = gl_window.window().inner_size();
                let (width, height) = (window_size.width as i32, window_size.height as i32);
                let capacity: usize = match TryInto::<usize>::try_into(width * height) {
                    Ok(capacity) => capacity * 4,
                    Err(_) => {
                        println!("meow");
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                        return ();
                    }
                };

                let mut bytes: Vec<u8> = Vec::with_capacity(capacity);
                let pixels = PixelPackData::Slice(&mut bytes);

                unsafe {
                    gl.read_pixels(0, 0, width, height, 0x1908, 0x1401, pixels);
                }

                let frame_builder = ndi_sdk::send::create_ndi_send_video_frame(
                    width,
                    height,
                    ndi_sdk::send::FrameFormatType::Progressive,
                )
                .with_data(
                    bytes,
                    width * 4,
                    ndi_sdk::send::SendColorFormat::Rgba,
                );

                let frame = match frame_builder.build() {
                    Ok(f) => f,
                    Err(_) => {
                        println!("whoops");
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                        return ();
                    }
                };

                sender.send_video(frame);

                // draw things on top of egui here

                gl_window.swap_buffers().unwrap();
            }
            ()
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                use glutin::event::WindowEvent;
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }

                if let glutin::event::WindowEvent::Resized(physical_size) = event {
                    gl_window.resize(physical_size);
                }

                egui_glow.on_event(&event);

                gl_window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
                ()
            }
            glutin::event::Event::LoopDestroyed => {
                egui_glow.destroy(&gl);
                ()
            }

            _ => (),
        }
    });
}
