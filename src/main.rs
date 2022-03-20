use std::fs::File;
use std::io::Read;

use futures::future::join3;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;

use tmbf::error::Result;
use tmbf::irc::{ComponentMessage, IrcCore, JoinChannelMessage};
use tmbf::ndi::{NDIFrameData, NDIPainter};
use tmbf::commander::Commander;

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

fn glutin_event_loop() -> Result<()> {
    let mut ndi_painter = NDIPainter::new()?;

    // egui/glow stuff
    let mut clear_color = [0.1, 0.1, 0.1];

    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let (gl_window, gl) = create_display(&event_loop);

    let mut egui_glow = egui_glow::EguiGlow::new(gl_window.window(), &gl);

    event_loop.run(
        move |event, _, control_flow: &mut glutin::event_loop::ControlFlow| {
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

                    // draw things on top of egui here

                    // get window size
                    let window_size = gl_window.window().inner_size();

                    // prep NDI video frame
                    let mut frame_data: NDIFrameData =
                        match (window_size.width as i32, window_size.height as i32).try_into() {
                            Ok(fd) => fd,
                            Err(_) => {
                                *control_flow = glutin::event_loop::ControlFlow::Exit;
                                return ();
                            }
                        };
                    frame_data.get_pixels(&gl);

                    // send NDI video frame
                    match ndi_painter.paint(frame_data) {
                        Err(_) => {
                            *control_flow = glutin::event_loop::ControlFlow::Exit;
                            return ();
                        }
                        _ => (),
                    };

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
        },
    );
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut file = File::open("/home/wayne/.config/twitchy-mcbotface/auth.yml")?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;
    let login_creds: StaticLoginCredentials = serde_yaml::from_str(&contents)?;

    let config = ClientConfig::new_simple(login_creds);
    let mut core = IrcCore::new();
    let join_dispatcher = core.get_msg_dispatcher();

    let run_irc_handle = core.run_irc(config);

    let cmdr_dispatcher = join_dispatcher.clone();
    let mut cmdr = Commander::new("TODO".to_string(), cmdr_dispatcher);
    let cmdr_handle = cmdr.run_commander();


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

    let (_, run_irc_result, joiner_result) = join3(cmdr_handle, run_irc_handle, joiner_handler).await;
    match joiner_result {
        Err(e) => {
            println!("joiner failed: {}", e)
        },
        _ => (),
    };
    match run_irc_result {
        Err(e) => {
            println!("run_irc failed: {}", e)
        },
        _ => (),
    };
    Ok(())
}
