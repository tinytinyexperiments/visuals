use std::time::Instant;

use wgpu::util::DeviceExt;
use winit::{dpi::LogicalSize, event::*, event_loop::EventLoop};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    time: f32,
    _pad: [f32; 3],
}

const SHADER: &str = r#"
struct Globals {
    time: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    var out: VSOut;
    let pos = positions[vi];
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = (pos + vec2<f32>(1.0, 1.0)) * 0.5;
    return out;
}

fn rot_y(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c,
    );
}

fn nugget_sdf(p: vec3<f32>, t: f32) -> f32 {
    // rotate the nugget over time
    let r = rot_y(t * 0.7);
    var q = r * p;

    // base blobby sphere
    var d = length(q) - 0.8;

    // a few lumpy bits
    d = min(d, length(q - vec3<f32>(0.35, 0.15, 0.1)) - 0.35);
    d = min(d, length(q - vec3<f32>(-0.3, -0.2, 0.2)) - 0.3);
    d = min(d, length(q - vec3<f32>(0.1, 0.25, -0.25)) - 0.28);

    // small sinusoidal roughness to feel crunchy
    let rough = 0.08 * (sin(q.x * 8.0) * sin(q.y * 9.0) * sin(q.z * 7.0));
    d = d + rough;

    return d;
}

fn map_scene(p: vec3<f32>, t: f32) -> f32 {
    return nugget_sdf(p, t);
}

fn estimate_normal(p: vec3<f32>, t: f32) -> vec3<f32> {
    let e = 0.001;
    let d = map_scene(p, t);
    let nx = map_scene(p + vec3<f32>(e, 0.0, 0.0), t) - d;
    let ny = map_scene(p + vec3<f32>(0.0, e, 0.0), t) - d;
    let nz = map_scene(p + vec3<f32>(0.0, 0.0, e), t) - d;
    return normalize(vec3<f32>(nx, ny, nz));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // normalized screen coordinates
    let uv = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let aspect = 800.0 / 600.0;
    let p = vec2<f32>(uv.x * aspect, uv.y);

    let t = globals.time;

    // camera setup
    let ro = vec3<f32>(0.0, 0.2, 3.0);
    let rd = normalize(vec3<f32>(p.x, p.y, -1.8));

    // raymarch
    var dist = 0.0;
    var hit = false;
    var pos = ro;

    for (var i: i32 = 0; i < 96; i = i + 1) {
        pos = ro + rd * dist;
        let d = map_scene(pos, t);
        if d < 0.002 {
            hit = true;
            break;
        }
        dist = dist + d;
        if dist > 8.0 {
            break;
        }
    }

    var col = vec3<f32>(0.02, 0.0, 0.06);

    if hit {
        let n = estimate_normal(pos, t);

        let light_dir = normalize(vec3<f32>(-0.4, 0.7, 0.3));
        let diff = max(dot(n, light_dir), 0.0);

        // simple fake subsurface / bounce from below
        let subsurf = max(dot(n, vec3<f32>(0.0, -1.0, 0.0)), 0.0);

        // crunchy nugget base color
        let base = vec3<f32>(0.85, 0.55, 0.2);

        let nugget = base * (0.25 + 0.85 * diff) + vec3<f32>(0.3, 0.15, 0.05) * subsurf;

        // slight rim light
        let view_dir = normalize(ro - pos);
        let rim = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0);

        col = nugget + rim * vec3<f32>(1.0, 0.8, 0.5);
    } else {
        // background gradient
        let y = p.y * 0.5 + 0.5;
        col = mix(
            vec3<f32>(0.02, 0.0, 0.05),
            vec3<f32>(0.1, 0.0, 0.15),
            y
        );
    }

    // clamp and slight gamma
    col = min(col, vec3<f32>(1.0, 1.0, 1.0));
    col = pow(col, vec3<f32>(0.8, 0.8, 0.8));

    return vec4<f32>(col, 1.0);
}
"#;

struct State<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    start_instant: Instant,
}

impl<'window> State<'window> {
    async fn new(window: &'window winit::window::Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapters found on the system!");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
            })
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Globals uniform buffer
        let globals = Globals {
            time: 0.0,
            _pad: [0.0; 3],
        };

        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globals Buffer"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Globals BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globals BG"),
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neon Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fullscreen Triangle Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            globals_buffer,
            globals_bind_group,
            start_instant: Instant::now(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let elapsed = self.start_instant.elapsed().as_secs_f32();
        let globals = Globals {
            time: elapsed,
            _pad: [0.0; 3],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop
        .create_window(
            winit::window::WindowAttributes::default()
                .with_title("wgpu playground - neon fractal")
                .with_inner_size(LogicalSize::new(800.0, 600.0)),
        )
        .unwrap();

    let mut state = pollster::block_on(State::new(&window));

    event_loop
        .run(|event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => state.resize(size),
                WindowEvent::ScaleFactorChanged { .. } => {
                    // We'll get a Resized event as well; handle resize there.
                }
                _ => {}
            },
            Event::AboutToWait => {
                match state.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = window.inner_size();
                        state.resize(size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!("Out of memory, exiting");
                        elwt.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        eprintln!("Surface timeout");
                    }
                    Err(wgpu::SurfaceError::Other) => {
                        eprintln!("Surface error: Other");
                    }
                }
                window.request_redraw();
            }
            _ => {}
        })
        .unwrap();
}

