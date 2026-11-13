//! The GPU mirror: a wgpu compute pass that shows where a pair differs.
//!
//! This crate is for display only. It never writes a verdict.
//! The CPU engine in `chrys-core` decides each verdict, and this crate does
//! not change that decision. The type system holds this rule: no function
//! here returns a `Verdict`.
//!
//! The compute pass finds the largest channel difference for each pixel.
//! It does not register the pair first, so a shifted pair shows a larger
//! area than the engine reports. The equivalence check in `agree` reads
//! a pair that needs no shift.

#![forbid(unsafe_code)]

use chrys_core::{BoundingBox, Verdict};
use chrys_source::Frame;
use wgpu::util::DeviceExt;

pub mod label;

/// The smallest difference that counts as a change.
///
/// This is the same value as the engine threshold, so the GPU mask and the
/// CPU regions use one meaning of "changed".
pub const THRESHOLD: u8 = chrys_core::classify::label::RESIDUAL_THRESHOLD;

const SHADER: &str = r#"
struct Size { width: u32, height: u32 }

@group(0) @binding(0) var<storage, read> base: array<u32>;
@group(0) @binding(1) var<storage, read> candidate: array<u32>;
@group(0) @binding(2) var<storage, read_write> difference: array<u32>;
@group(0) @binding(3) var<uniform> size: Size;

fn channel(value: u32, shift: u32) -> i32 {
    return i32((value >> shift) & 0xffu);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= size.width || id.y >= size.height) {
        return;
    }
    let index = id.y * size.width + id.x;
    let a = base[index];
    let b = candidate[index];
    var largest = 0;
    for (var shift = 0u; shift < 32u; shift = shift + 8u) {
        largest = max(largest, abs(channel(a, shift) - channel(b, shift)));
    }
    difference[index] = u32(largest);
}
"#;

/// The error that the GPU mirror returns.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// No adapter is available on this machine.
    #[error("no GPU adapter is available: {0}")]
    NoAdapter(String),
    /// The adapter refused the device request.
    #[error("the GPU adapter refused a device: {0}")]
    NoDevice(String),
    /// The two frames have different sizes.
    #[error("the frames differ in size: {base:?} and {candidate:?}")]
    ShapeMismatch {
        /// The base size.
        base: (u32, u32),
        /// The candidate size.
        candidate: (u32, u32),
    },
    /// A frame is larger than the storage buffer limit of the adapter.
    #[error("a frame needs {needed} bytes, and the adapter limit is {limit} bytes")]
    TooLarge {
        /// The bytes that one frame needs.
        needed: u64,
        /// The limit of the adapter.
        limit: u64,
    },
    /// The read back from the GPU failed.
    #[error("the GPU read back failed: {0}")]
    ReadBack(String),
}

/// The name and the vendor of the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReport {
    /// The adapter name.
    pub name: String,
    /// The PCI vendor id. The Metal backend reports 0.
    pub vendor: u32,
    /// The backend, for example Metal or Vulkan.
    pub backend: String,
}

/// The per-pixel difference that the GPU calculated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferenceMask {
    /// The width, in pixels.
    pub width: u32,
    /// The height, in pixels.
    pub height: u32,
    /// The largest channel difference for each pixel, row major.
    pub values: Vec<u8>,
}

impl DifferenceMask {
    /// Return the bounding box of each 8-connected group of pixels whose
    /// difference is more than `THRESHOLD`.
    pub fn changed_boxes(&self) -> Vec<BoundingBox> {
        let changed: Vec<bool> = self.values.iter().map(|&v| v > THRESHOLD).collect();
        label::boxes(&changed, self.width, self.height)
    }
}

/// A wgpu device and the compute pipeline of the mirror.
pub struct GpuMirror {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    report: AdapterReport,
    max_binding: u64,
}

impl GpuMirror {
    /// Open the default adapter and make the compute pipeline.
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(|error| GpuError::NoAdapter(error.to_string()))?;
        Self::with_adapter(&adapter)
    }

    /// Make the compute pipeline on a device from `adapter`.
    pub fn with_adapter(adapter: &wgpu::Adapter) -> Result<Self, GpuError> {
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("chrys-gpu"),
            required_limits: limits.clone(),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| GpuError::NoDevice(error.to_string()))?;
        let info = adapter.get_info();
        let report = AdapterReport {
            name: info.name,
            vendor: info.vendor,
            backend: format!("{:?}", info.backend),
        };
        Ok(Self::from_device(device, queue, report, limits))
    }

    /// Make the compute pipeline on a device that a caller already holds.
    ///
    /// The window uses this, so one device serves the display and the mirror.
    pub fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        report: AdapterReport,
        limits: wgpu::Limits,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("difference"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("difference"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let max_binding = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        GpuMirror {
            device,
            queue,
            pipeline,
            report,
            max_binding,
        }
    }

    /// Return the name and the vendor of the adapter.
    pub fn report(&self) -> &AdapterReport {
        &self.report
    }

    /// Calculate the difference mask of `base` and `candidate` on the GPU.
    ///
    /// The result goes to the CPU through a separate staging buffer. The
    /// compute buffer is never mapped directly.
    pub fn difference(&self, base: &Frame, candidate: &Frame) -> Result<DifferenceMask, GpuError> {
        if !base.same_shape_as(candidate) {
            return Err(GpuError::ShapeMismatch {
                base: (base.width, base.height),
                candidate: (candidate.width, candidate.height),
            });
        }
        let pixels = base.pixel_count() as u64;
        let bytes = pixels * 4;
        if bytes > self.max_binding {
            return Err(GpuError::TooLarge {
                needed: bytes,
                limit: self.max_binding,
            });
        }

        let storage = |label, contents: &[u8]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };
        let base_buffer = storage("base", base.rgba8());
        let candidate_buffer = storage("candidate", candidate.rgba8());
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("difference"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let size: [u32; 2] = [base.width, base.height];
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("size"),
                contents: bytemuck::cast_slice(&size),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("difference"),
            layout: &layout,
            entries: &[
                entry(0, &base_buffer),
                entry(1, &candidate_buffer),
                entry(2, &output),
                entry(3, &uniform),
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(base.width.div_ceil(8), base.height.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, bytes);
        self.queue.submit([encoder.finish()]);

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| GpuError::ReadBack(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| GpuError::ReadBack(error.to_string()))?
            .map_err(|error| GpuError::ReadBack(error.to_string()))?;

        let values = {
            let view = slice
                .get_mapped_range()
                .map_err(|error| GpuError::ReadBack(format!("{error:?}")))?;
            let words: &[u32] = bytemuck::cast_slice(&view);
            // The shader writes a value from 0 to 255 in each word.
            words.iter().map(|&word| word.min(255) as u8).collect()
        };
        staging.unmap();
        Ok(DifferenceMask {
            width: base.width,
            height: base.height,
            values,
        })
    }
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

/// The result of the categorical equivalence check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Agreement {
    /// The GPU boxes and the CPU regions agree.
    Agree,
    /// The two sides find a different number of changed areas.
    CountDiffers {
        /// The number of CPU regions.
        cpu: usize,
        /// The number of GPU boxes.
        gpu: usize,
    },
    /// One CPU region has no GPU box inside the tolerance.
    BoxDiffers {
        /// The CPU box.
        cpu: BoundingBox,
    },
    /// The CPU refused the pair, so no categorical check applies.
    CpuRefused,
}

/// Check the GPU boxes against the CPU verdict by category.
///
/// The two sides agree when they find the same number of changed areas,
/// and each CPU region has a GPU box whose four edges are each inside
/// `tolerance` pixels. The GPU does not classify a kind. The kind comes
/// from the CPU only, so this check does not compare kinds.
pub fn agree(cpu: &Verdict, gpu: &[BoundingBox], tolerance: u32) -> Agreement {
    let regions = match cpu {
        Verdict::Identical => &[][..],
        Verdict::Changed { regions } => regions.as_slice(),
        Verdict::Refused { .. } => return Agreement::CpuRefused,
    };
    if regions.len() != gpu.len() {
        return Agreement::CountDiffers {
            cpu: regions.len(),
            gpu: gpu.len(),
        };
    }
    for region in regions {
        if !gpu
            .iter()
            .any(|candidate| near(&region.bbox, candidate, tolerance))
        {
            return Agreement::BoxDiffers { cpu: region.bbox };
        }
    }
    Agreement::Agree
}

fn near(a: &BoundingBox, b: &BoundingBox, tolerance: u32) -> bool {
    let edges = |bbox: &BoundingBox| {
        [
            i64::from(bbox.x),
            i64::from(bbox.y),
            i64::from(bbox.x) + i64::from(bbox.width),
            i64::from(bbox.y) + i64::from(bbox.height),
        ]
    };
    edges(a)
        .iter()
        .zip(edges(b))
        .all(|(left, right)| (left - right).abs() <= i64::from(tolerance))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrys_core::{ChangeKind, Region};

    fn bbox(x: u32, y: u32, width: u32, height: u32) -> BoundingBox {
        BoundingBox {
            x,
            y,
            width,
            height,
        }
    }

    fn changed(boxes: &[BoundingBox]) -> Verdict {
        Verdict::Changed {
            regions: boxes
                .iter()
                .map(|&bbox| Region {
                    kind: ChangeKind::Recoloured,
                    bbox,
                    offset_px: None,
                    colour_delta: None,
                })
                .collect(),
        }
    }

    #[test]
    fn identical_agrees_with_no_box() {
        assert_eq!(agree(&Verdict::Identical, &[], 2), Agreement::Agree);
    }

    #[test]
    fn a_box_inside_the_tolerance_agrees() {
        let cpu = changed(&[bbox(10, 10, 20, 20)]);
        assert_eq!(agree(&cpu, &[bbox(12, 9, 19, 22)], 2), Agreement::Agree);
    }

    #[test]
    fn a_box_outside_the_tolerance_does_not_agree() {
        let cpu = changed(&[bbox(10, 10, 20, 20)]);
        assert_eq!(
            agree(&cpu, &[bbox(14, 10, 20, 20)], 2),
            Agreement::BoxDiffers {
                cpu: bbox(10, 10, 20, 20)
            }
        );
    }

    #[test]
    fn a_different_count_does_not_agree() {
        let cpu = changed(&[bbox(0, 0, 4, 4)]);
        assert_eq!(
            agree(&cpu, &[bbox(0, 0, 4, 4), bbox(9, 9, 2, 2)], 2),
            Agreement::CountDiffers { cpu: 1, gpu: 2 }
        );
    }
}
