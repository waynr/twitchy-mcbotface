use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use glutin::event_loop::EventLoop;
use glutin::event_loop::EventLoopProxy;
use tokio::sync::mpsc;
use epaint::text::{FontData, FontDefinitions, FontFamily};

use crate::egui_ui::Chatbox;
use crate::egui_ui::ChatboxState;
use crate::error::Result;
use crate::ndi::NDIFrameData;

pub enum BotfaceEvent {
    Nonce,
}

pub struct Botface {
    chatbox: Chatbox,
    event_loop: EventLoop<BotfaceEvent>,
    frame_sender: mpsc::UnboundedSender<NDIFrameData>,
}

impl Botface {
    pub fn new(frame_sender: mpsc::UnboundedSender<NDIFrameData>) -> Result<Self> {
        let event_loop = glutin::event_loop::EventLoop::<BotfaceEvent>::with_user_event();
        let chatbox_state = Arc::new(Mutex::new(ChatboxState::new()));
        let chatbox = Chatbox::new(chatbox_state);
        Ok(Self {
            chatbox,
            event_loop,
            frame_sender,
        })
    }

    pub fn chatbox_state(&self) -> Arc<Mutex<ChatboxState>> {
        self.chatbox.state()
    }

    pub fn event_loop_proxy(&self) -> EventLoopProxy<BotfaceEvent> {
        self.event_loop.create_proxy()
    }

    pub fn run_event_loop(self) -> Result<()> {
        run_event_loop(self.event_loop, self.chatbox, self.frame_sender)
    }
}

fn create_display(
    event_loop: &glutin::event_loop::EventLoop<BotfaceEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(false)
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

pub fn run_event_loop(
    event_loop: glutin::event_loop::EventLoop<BotfaceEvent>,
    mut chatbox: Chatbox,
    frame_sender: mpsc::UnboundedSender<NDIFrameData>,
) -> Result<()> {
    // egui/glow stuff
    let mut clear_color = [0.1, 0.1, 0.1];

    let (gl_window, gl) = create_display(&event_loop);
    let rc_gl = Rc::new(gl);

    let mut egui_glow = egui_glow::EguiGlow::new(gl_window.window(), rc_gl.clone());

    let mut font_definitions = FontDefinitions::default();
    font_definitions.font_data.insert(
        "hack-regular".to_owned(),
        FontData::from_static(include_bytes!(
            //"/usr/share/fonts/truetype/hack/Hack-Regular.ttf"
            //"/usr/share/fonts/fonts-go/Go-Mono.ttf"
            //"/usr/share/fonts/truetype/monoid/Monoid-Regular.ttf"
            "/usr/share/fonts/truetype/firacode/FiraCode-Regular.ttf"
        )),
    );
    font_definitions
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "hack-regular".to_owned());
    egui_glow.egui_ctx.set_fonts(font_definitions);

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
                    let chatbox_context = egui_ctx.clone();
                    egui::Window::new("chat box").show(egui_ctx, |ui| {
                        chatbox.show(ui, chatbox_context);
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
                        rc_gl.clear_color(clear_color[0], clear_color[1], clear_color[2], 1.0);
                        rc_gl.clear(glow::COLOR_BUFFER_BIT);
                    }

                    // draw things behind egui here

                    egui_glow.paint(gl_window.window());

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
                    frame_data.get_pixels(&rc_gl);

                    // send NDI video frame to async NDIPainter
                    match frame_sender.send(frame_data) {
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

                // BotFaceEvent received from some botface component like ChatBoxDispatcher
                glutin::event::Event::UserEvent(_) => redraw(),

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
                    egui_glow.destroy();
                    ()
                }

                _ => (),
            }
        },
    );
}
