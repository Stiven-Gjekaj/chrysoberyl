//! The native window: a person reviews a pair here.
//!
//! The window shows a pair side by side, as an overlay, or as a difference
//! map. A person pans, zooms, moves through the frames, and accepts or
//! rejects the candidate.
//!
//! The GPU draws the window and the difference map. The verdicts come from
//! the CPU engine, and the window only shows them. `chrys-gpu` holds the
//! same rule.
//!
//! Keys:
//!
//! - `1`, `2`, `3`: side by side, overlay, difference.
//! - Left and Right arrows: the previous and the next frame.
//! - Space: show the base or the candidate in the overlay.
//! - `[` and `]`: change the overlay blend.
//! - `+`, `-`, the mouse wheel: zoom. A drag with the left button: pan.
//! - `0`: fit the image in the window.
//! - `a`: accept the candidate as the new baseline. `r`: reject it.
//! - `q`, Escape: close the window.

#![forbid(unsafe_code)]

pub mod state;

use std::sync::Arc;

use chrys_core::Verdict;
use chrys_gpu::{AdapterReport, GpuMirror};
use chrys_source::Frame;
use state::{Action, Decision, Mode, ViewState};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The pair that the window shows.
pub struct ViewPair {
    /// The base frames.
    pub base: Vec<Frame>,
    /// The candidate frames.
    pub candidate: Vec<Frame>,
    /// The name of the source file of each base frame.
    pub names: Vec<String>,
    /// The CPU verdict of each frame pair.
    pub verdicts: Vec<Verdict>,
}

/// The function that writes the new baseline. It returns a message for the
/// person, or an error message.
pub type AcceptFn = Box<dyn FnMut() -> Result<String, String>>;

/// The error that the window returns.
#[derive(Debug)]
pub struct ViewError(pub String);

impl std::fmt::Display for ViewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ViewError {}

/// Open the window and show `pair`. Return the last decision of the
/// person when the window closes.
pub fn run(pair: ViewPair, accept: Option<AcceptFn>) -> Result<Option<Decision>, ViewError> {
    if pair.base.is_empty() || pair.base.len() != pair.candidate.len() {
        return Err(ViewError(format!(
            "the window needs the same number of frames on each side, and got {} and {}",
            pair.base.len(),
            pair.candidate.len()
        )));
    }
    let event_loop = EventLoop::new().map_err(|error| ViewError(error.to_string()))?;
    let mut app = App {
        state: ViewState::new(pair.base.len()),
        pair,
        accept,
        gpu: None,
        message: String::new(),
        cursor: (0.0, 0.0),
        dragging: false,
        error: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| ViewError(error.to_string()))?;
    match app.error {
        Some(error) => Err(error),
        None => Ok(app.state.decision),
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    window: [f32; 2],
    zoom: f32,
    blend: f32,
    pan: [f32; 2],
    base_size: [f32; 2],
    candidate_size: [f32; 2],
    mode: u32,
    threshold: f32,
}

const SHADER: &str = r#"
struct View {
    window: vec2<f32>,
    zoom: f32,
    blend: f32,
    pan: vec2<f32>,
    base_size: vec2<f32>,
    candidate_size: vec2<f32>,
    mode: u32,
    threshold: f32,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var base: texture_2d<f32>;
@group(0) @binding(2) var candidate: texture_2d<f32>;
@group(0) @binding(3) var difference: texture_2d<f32>;

@vertex
fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn checker(position: vec2<f32>) -> vec3<f32> {
    let cell = (u32(position.x / 8.0) + u32(position.y / 8.0)) & 1u;
    return select(vec3<f32>(0.20), vec3<f32>(0.26), cell == 1u);
}

fn inside(point: vec2<f32>, size: vec2<f32>) -> bool {
    return point.x >= 0.0 && point.y >= 0.0 && point.x < size.x && point.y < size.y;
}

fn fetch(texture: texture_2d<f32>, point: vec2<f32>, size: vec2<f32>, screen: vec2<f32>) -> vec3<f32> {
    if (!inside(point, size)) {
        return vec3<f32>(0.08);
    }
    let colour = textureLoad(texture, vec2<i32>(floor(point)), 0);
    return mix(checker(screen), colour.rgb, colour.a);
}

@fragment
fn fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    var pane = view.window.x;
    var origin = 0.0;
    var right = false;
    if (view.mode == 0u) {
        pane = view.window.x * 0.5;
        if (abs(position.x - pane) < 1.0) {
            return vec4<f32>(0.5, 0.5, 0.5, 1.0);
        }
        if (position.x >= pane) {
            origin = pane;
            right = true;
        }
    }
    let relative = vec2<f32>(position.x - origin - pane * 0.5, position.y - view.window.y * 0.5);
    let point = (relative - view.pan) / view.zoom;
    let base_point = point + view.base_size * 0.5;
    let candidate_point = point + view.candidate_size * 0.5;

    if (view.mode == 0u) {
        if (right) {
            return vec4<f32>(fetch(candidate, candidate_point, view.candidate_size, position.xy), 1.0);
        }
        return vec4<f32>(fetch(base, base_point, view.base_size, position.xy), 1.0);
    }
    if (view.mode == 1u) {
        let a = fetch(base, base_point, view.base_size, position.xy);
        let b = fetch(candidate, candidate_point, view.candidate_size, position.xy);
        return vec4<f32>(mix(a, b, view.blend), 1.0);
    }
    let a = fetch(base, base_point, view.base_size, position.xy);
    let grey = vec3<f32>(dot(a, vec3<f32>(0.2126, 0.7152, 0.0722)) * 0.6 + 0.2);
    if (inside(base_point, view.base_size)) {
        let d = textureLoad(difference, vec2<i32>(floor(base_point)), 0).r;
        if (d > view.threshold) {
            return vec4<f32>(mix(vec3<f32>(1.0, 0.1, 0.1), grey, 0.25), 1.0);
        }
    }
    return vec4<f32>(grey, 1.0);
}
"#;

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    mirror: GpuMirror,
    loaded_frame: Option<usize>,
    note: String,
}

struct App {
    state: ViewState,
    pair: ViewPair,
    accept: Option<AcceptFn>,
    gpu: Option<Gpu>,
    message: String,
    cursor: (f32, f32),
    dragging: bool,
    error: Option<ViewError>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(ViewError(error));
        event_loop.exit();
    }

    fn pane(&self) -> (f32, f32) {
        let (width, height) = self
            .gpu
            .as_ref()
            .map(|gpu| (gpu.config.width as f32, gpu.config.height as f32))
            .unwrap_or((1.0, 1.0));
        match self.state.mode {
            Mode::SideBySide => (width / 2.0, height),
            _ => (width, height),
        }
    }

    /// The cursor position relative to the centre of the pane under it.
    fn cursor_in_pane(&self) -> (f32, f32) {
        let (pane_width, pane_height) = self.pane();
        let x = if self.cursor.0 >= pane_width && self.state.mode == Mode::SideBySide {
            self.cursor.0 - pane_width
        } else {
            self.cursor.0
        };
        (x - pane_width / 2.0, self.cursor.1 - pane_height / 2.0)
    }

    fn fit(&mut self) {
        let frame = &self.pair.base[self.state.frame];
        let pane = self.pane();
        self.state.fit((frame.width, frame.height), pane);
    }

    fn act(&mut self, action: Action) {
        match action {
            Action::Fit => self.fit(),
            Action::Decide(Decision::Accept) => {
                self.state.apply(action);
                self.message = match self.accept.as_mut() {
                    Some(accept) => match accept() {
                        Ok(message) => message,
                        Err(error) => format!("accept failed: {error}"),
                    },
                    None => "no baseline name, so nothing was written".to_string(),
                };
                println!("{}", self.message);
            }
            Action::Decide(Decision::Reject) => {
                self.state.apply(action);
                self.message = "rejected".to_string();
                println!("{}", self.message);
            }
            other => self.state.apply(other),
        }
        self.refresh_title();
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }

    fn refresh_title(&self) {
        let Some(gpu) = &self.gpu else { return };
        let index = self.state.frame;
        let verdict = match &self.pair.verdicts[index] {
            Verdict::Identical => "identical".to_string(),
            Verdict::Changed { regions } => format!("changed, {} region(s)", regions.len()),
            Verdict::Refused { .. } => "refused".to_string(),
        };
        let mut title = format!(
            "chrys view: frame {} of {} ({}), {}, {}",
            index + 1,
            self.state.frame_count,
            self.pair.names[index],
            self.state.mode.name(),
            verdict
        );
        if !gpu.note.is_empty() {
            title.push_str(&format!(", {}", gpu.note));
        }
        if !self.message.is_empty() {
            title.push_str(&format!(", {}", self.message));
        }
        gpu.window.set_title(&title);
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attributes = Window::default_attributes()
            .with_title("chrys view")
            .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|error| error.to_string())?,
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| error.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        }))
        .map_err(|error| error.to_string())?;
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("chrys-view"),
            required_limits: limits.clone(),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| error.to_string())?;
        let info = adapter.get_info();
        let report = AdapterReport {
            name: info.name,
            vendor: info.vendor,
            backend: format!("{:?}", info.backend),
        };
        let mirror = GpuMirror::from_device(device.clone(), queue.clone(), report, limits);

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("the surface does not support this adapter")?;
        let capabilities = surface.get_capabilities(&adapter);
        if let Some(format) = capabilities.formats.iter().find(|format| format.is_srgb()) {
            config.format = *format;
        }
        surface.configure(&device, &config);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("view"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("view"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("view"),
            contents: bytemuck::bytes_of(&Uniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        self.gpu = Some(Gpu {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            uniforms,
            bind_group: None,
            mirror,
            loaded_frame: None,
            note: String::new(),
        });
        self.fit();
        self.refresh_title();
        Ok(())
    }

    /// Upload the textures of the frame on view, when it changed.
    fn load_frame(&mut self) {
        let index = self.state.frame;
        let Some(gpu) = self.gpu.as_mut() else { return };
        if gpu.loaded_frame == Some(index) {
            return;
        }
        let base = &self.pair.base[index];
        let candidate = &self.pair.candidate[index];
        let (mask, note) = match gpu.mirror.difference(base, candidate) {
            Ok(mask) => (mask.values, String::new()),
            Err(error) => (
                vec![0; base.pixel_count()],
                format!("no difference map: {error}"),
            ),
        };
        gpu.note = note;
        let base_view = texture(gpu, base.width, base.height, base.rgba8(), Format::Colour);
        let candidate_view = texture(
            gpu,
            candidate.width,
            candidate.height,
            candidate.rgba8(),
            Format::Colour,
        );
        let mask_view = texture(gpu, base.width, base.height, &mask, Format::Mask);
        let layout = gpu.pipeline.get_bind_group_layout(0);
        gpu.bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&candidate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&mask_view),
                },
            ],
        }));
        gpu.loaded_frame = Some(index);
        self.refresh_title();
    }

    fn draw(&mut self) {
        self.load_frame();
        let index = self.state.frame;
        let base = &self.pair.base[index];
        let candidate = &self.pair.candidate[index];
        let Some(gpu) = self.gpu.as_mut() else { return };
        let uniforms = Uniforms {
            window: [gpu.config.width as f32, gpu.config.height as f32],
            zoom: self.state.zoom,
            blend: self.state.blend,
            pan: [self.state.pan.0, self.state.pan.1],
            base_size: [base.width as f32, base.height as f32],
            candidate_size: [candidate.width as f32, candidate.height as f32],
            mode: self.state.mode.code(),
            threshold: f32::from(chrys_gpu::THRESHOLD) / 255.0,
        };
        gpu.queue
            .write_buffer(&gpu.uniforms, 0, bytemuck::bytes_of(&uniforms));

        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                gpu.window.request_redraw();
                return;
            }
            _ => return,
        };
        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("view"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&gpu.pipeline);
            if let Some(bind_group) = &gpu.bind_group {
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
        gpu.window.pre_present_notify();
        gpu.queue.present(frame);
    }
}

enum Format {
    Colour,
    Mask,
}

fn texture(gpu: &Gpu, width: u32, height: u32, data: &[u8], format: Format) -> wgpu::TextureView {
    let (texture_format, bytes_per_pixel) = match format {
        // The frame holds sRGB values. The sRGB format makes the shader
        // read linear values, and the sRGB surface writes them back.
        Format::Colour => (wgpu::TextureFormat::Rgba8UnormSrgb, 4),
        Format::Mask => (wgpu::TextureFormat::R8Unorm, 1),
    };
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture_format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes_per_pixel),
            rows_per_image: Some(height),
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

use bytemuck::Zeroable;

/// Return the action for a key, or `None`.
fn key_action(key: &Key) -> Option<Action> {
    match key {
        Key::Named(NamedKey::ArrowRight) => Some(Action::NextFrame),
        Key::Named(NamedKey::ArrowLeft) => Some(Action::PreviousFrame),
        Key::Named(NamedKey::Space) => Some(Action::Flip),
        Key::Character(text) => match text.as_str() {
            "1" => Some(Action::SetMode(Mode::SideBySide)),
            "2" => Some(Action::SetMode(Mode::Overlay)),
            "3" => Some(Action::SetMode(Mode::Difference)),
            "[" => Some(Action::Blend(-0.1)),
            "]" => Some(Action::Blend(0.1)),
            "+" | "=" => Some(Action::ZoomAt {
                factor: 1.25,
                x: 0.0,
                y: 0.0,
            }),
            "-" => Some(Action::ZoomAt {
                factor: 0.8,
                x: 0.0,
                y: 0.0,
            }),
            "0" => Some(Action::Fit),
            "a" | "A" => Some(Action::Decide(Decision::Accept)),
            "r" | "R" => Some(Action::Decide(Decision::Reject)),
            _ => None,
        },
        _ => None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_none()
            && let Err(error) = self.start(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.config.width = size.width.max(1);
                    gpu.config.height = size.height.max(1);
                    gpu.surface.configure(&gpu.device, &gpu.config);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character(text) if text.as_str() == "q" => event_loop.exit(),
                    key => {
                        if let Some(action) = key_action(key) {
                            self.act(action);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let next = (position.x as f32, position.y as f32);
                if self.dragging {
                    self.act(Action::Pan(next.0 - self.cursor.0, next.1 - self.cursor.1));
                }
                self.cursor = next;
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.dragging = state == ElementState::Pressed,
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 40.0,
                };
                let (x, y) = self.cursor_in_pane();
                self.act(Action::ZoomAt {
                    factor: 1.15f32.powf(steps),
                    x,
                    y,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_map_to_their_actions() {
        let character = |text: &str| Key::Character(text.into());
        assert_eq!(
            key_action(&character("3")),
            Some(Action::SetMode(Mode::Difference))
        );
        assert_eq!(
            key_action(&Key::Named(NamedKey::ArrowRight)),
            Some(Action::NextFrame)
        );
        assert_eq!(
            key_action(&character("a")),
            Some(Action::Decide(Decision::Accept))
        );
        assert_eq!(key_action(&character("x")), None);
    }

    #[test]
    fn the_uniform_block_is_48_bytes() {
        // The WGSL struct `View` is 48 bytes. A change on one side only
        // moves every field after it.
        assert_eq!(std::mem::size_of::<Uniforms>(), 48);
    }
}
