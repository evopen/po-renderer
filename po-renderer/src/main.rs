#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused))]

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

mod engine;
mod profiler;

use std::collections::HashMap;

use backtrace::Backtrace;
use glam::vec3;
use glam::Vec3;

fn main() {
    run();
}

fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("debug,gpu_allocator=info")
        .init();
    dotenv::dotenv().ok();
    let event_loop = winit::event_loop::EventLoop::new();

    let mut windows = HashMap::new();
    let main_window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    // let profiler_window = winit::window::WindowBuilder::new()
    //     .with_inner_size(winit::dpi::PhysicalSize::new(640, 600))
    //     .build(&event_loop)
    //     .unwrap();
    let mut engine = engine::Engine::new(&main_window);
    let main_window_id = main_window.id();

    windows.insert(main_window_id, main_window);

    // let mut profiler = profiler::Profiler::new(&profiler_window);
    puffin::set_scopes_on(false);

    event_loop.run(move |event, event_loop, control_flow| {
        // puffin::GlobalProfiler::lock().new_frame();
        *control_flow = winit::event_loop::ControlFlow::Poll;

        engine.update(&event);
        // profiler.update(&event);
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        windows.remove(&window_id);
                        if windows.is_empty() {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                        }
                    }
                    _ => {}
                }
            }
            winit::event::Event::MainEventsCleared => {
                if let Some(main_window) = windows.get(&main_window_id) {
                    main_window.request_redraw();
                }
            }
            winit::event::Event::RedrawRequested(_) => {
                // puffin::profile_scope!("render!");
                engine.render();
                // profiler.render();
            }
            winit::event::Event::DeviceEvent { device_id, event } => {
                match event {
                    winit::event::DeviceEvent::Key(input) => {
                        match input.virtual_keycode {
                            Some(key_code) => {
                                match key_code {
                                    _ => {}
                                }
                            }
                            None => todo!(),
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
