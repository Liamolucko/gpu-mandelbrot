use gpu_mandelbrot::State;
use winit::dpi::LogicalPosition;
use winit::dpi::LogicalSize;
use winit::event::ElementState;
use winit::event::Event;
use winit::event::MouseButton;
use winit::event::MouseScrollDelta;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::Window;

fn main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();

        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        console_error_panic_hook::set_once();
        console_log::init().expect("could not initialize logger");

        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut state = State::new(&window).await;

    // The mouse's offset in logical pixels from the center of the window.
    let mut mouse_offset = [0.0, 0.0];
    let mut dragging = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    state.resize(size.width, size.height);
                    // At least on macOS, it doesn't seem like resizing triggers redraws on its own.
                    window.request_redraw();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let scale_factor = window.scale_factor();
                    let position: LogicalPosition<f32> = position.to_logical(scale_factor);
                    let size: LogicalSize<f32> = window.inner_size().to_logical(scale_factor);

                    let x_offset = position.x - size.width / 2.0;
                    let y_offset = -(position.y - size.height / 2.0);

                    if dragging {
                        let x_delta = x_offset - mouse_offset[0];
                        let y_delta = y_offset - mouse_offset[1];
                        state.camera = [
                            state.camera[0] - x_delta / state.zoom,
                            state.camera[1] - y_delta / state.zoom,
                        ];

                        state.update_camera();
    
                        window.request_redraw();
                    }

                    mouse_offset = [x_offset, y_offset];
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let scrolled = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 60.0,
                    };

                    // The old offset of the mouse from the camera in the complex plane.
                    let old_offset = [mouse_offset[0] / state.zoom, mouse_offset[1] / state.zoom];

                    state.zoom *= 1.1f32.powf(scrolled);
                    // TODO: clamp zoom

                    // The new offset of the mouse from the camera in the complex plane.
                    let new_offset = [mouse_offset[0] / state.zoom, mouse_offset[1] / state.zoom];

                    let delta = [new_offset[0] - old_offset[0], new_offset[1] - old_offset[1]];

                    // Cancel out the change in the mouse's position on the complex plane.
                    // This means that as you zoom in, the mouse will stay in the same spot.
                    state.camera = [state.camera[0] - delta[0], state.camera[1] - delta[1]];

                    state.update_camera();

                    window.request_redraw();
                }
                WindowEvent::MouseInput { button, state, .. } => match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        dragging = true;
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        dragging = false;
                    }
                    _ => {}
                },
                _ => {}
            },
            Event::RedrawRequested(_) => state.render(),
            _ => {}
        }
    });
}
