#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused))]

mod engine;
mod profiler;

use glam::vec3a as vec3;
use glam::Vec3A as Vec3;

fn main() {
    tracing_subscriber::fmt().with_env_filter("debug").init();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();
    // let profiler_window = winit::window::WindowBuilder::new()
    //     .with_inner_size(winit::dpi::PhysicalSize::new(640, 600))
    //     .build(&event_loop)
    //     .unwrap();
    let mut engine = engine::Engine::new(&window);
    // let mut profiler = profiler::Profiler::new(&profiler_window);
    puffin::set_scopes_on(true);

    event_loop.run(move |event, _, control_flow| {
        // puffin::GlobalProfiler::lock().new_frame();
        *control_flow = winit::event_loop::ControlFlow::Poll;

        engine.update(&event);
        // profiler.update(&event);
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            winit::event::Event::MainEventsCleared => window.request_redraw(),
            winit::event::Event::RedrawRequested(_) => {
                // puffin::profile_scope!("render!");
                engine.render();
                // profiler.render();
            }
            _ => {}
        }
    });
}
