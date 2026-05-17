use crate::{FrameSize, Result, SoftwareBuffer};

const SHADER_SOURCE: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    switch vertex_index {
        case 0u: {
            output.position = vec4<f32>(-1.0,  1.0, 0.0, 1.0);
            output.uv = vec2<f32>(0.0, 0.0);
        }
        case 1u: {
            output.position = vec4<f32>(-1.0, -1.0, 0.0, 1.0);
            output.uv = vec2<f32>(0.0, 1.0);
        }
        case 2u: {
            output.position = vec4<f32>( 1.0,  1.0, 0.0, 1.0);
            output.uv = vec2<f32>(1.0, 0.0);
        }
        case 3u: {
            output.position = vec4<f32>( 1.0,  1.0, 0.0, 1.0);
            output.uv = vec2<f32>(1.0, 0.0);
        }
        case 4u: {
            output.position = vec4<f32>(-1.0, -1.0, 0.0, 1.0);
            output.uv = vec2<f32>(0.0, 1.0);
        }
        default: {
            output.position = vec4<f32>( 1.0, -1.0, 0.0, 1.0);
            output.uv = vec2<f32>(1.0, 1.0);
        }
    }
    return output;
}

@group(0) @binding(0)
var frame_texture: texture_2d<f32>;

@group(0) @binding(1)
var frame_sampler: sampler;

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame_texture, frame_sampler, input.uv);
}
"#;

#[derive(Debug)]
pub(super) struct TextureCompositor {
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    source_texture_format: wgpu::TextureFormat,
}

impl TextureCompositor {
    pub(super) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let source_texture_format = if surface_format.is_srgb() {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8Unorm
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("veila-gpu-frame-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("veila-gpu-frame-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("veila-gpu-frame-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("veila-gpu-frame-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("veila-gpu-frame-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            sampler,
            bind_group_layout,
            pipeline,
            source_texture_format,
        }
    }

    pub(super) fn upload_frame(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &SoftwareBuffer,
    ) -> GpuFrameTexture {
        let size = buffer.size();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("veila-gpu-frame-texture"),
            size: texture_extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.source_texture_format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            buffer.pixels(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: Some(size.height),
            },
            texture_extent(size),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("veila-gpu-frame-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        GpuFrameTexture {
            _texture: texture,
            _view: view,
            bind_group,
        }
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        frame_texture: &GpuFrameTexture,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("veila-gpu-frame-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &frame_texture.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

#[derive(Debug)]
pub(super) struct GpuFrameTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

pub(super) fn validate_upload_buffer(buffer: &SoftwareBuffer) -> Result<()> {
    let size = buffer.size();
    if size.is_empty() {
        return Err(crate::RendererError::EmptyFrame);
    }
    size.byte_len()
        .filter(|byte_len| *byte_len == buffer.pixels().len())
        .map(|_| ())
        .ok_or(crate::RendererError::InvalidFrameSize(size))
}

fn texture_extent(size: FrameSize) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.width.max(1),
        height: size.height.max(1),
        depth_or_array_layers: 1,
    }
}
