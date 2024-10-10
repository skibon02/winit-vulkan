use std::ffi::CStr;
use ash::Device;
use ash::vk::{ColorComponentFlags, CompareOp, CullModeFlags, DynamicState, GraphicsPipelineCreateInfo, Pipeline, PipelineCache, PipelineCacheCreateInfo, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, Rect2D, RenderPass, SampleCountFlags, ShaderModuleCreateInfo, ShaderStageFlags, Viewport};
use sparkles_macro::range_event_start;

pub struct VulkanPipeline {
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    pipeline_cache: PipelineCache,
}

#[macro_export]
macro_rules! use_shader {
    ($name:expr) => {
        (
            include_bytes!(concat!("../../shaders/compiled/", $name, "_vert.spv")),
            include_bytes!(concat!("../../shaders/compiled/", $name, "_frag.spv"))
        )
    };
}

pub struct PipelineDesc<'a> {
    vertex_shader_code: &'a [u8],
    fragment_shader_code: &'a [u8],
}

impl<'a> PipelineDesc<'a> {
    pub fn new((vertex_shader_code, fragment_shader_code): (&'a [u8], &'a [u8])) -> PipelineDesc<'a> {
        Self {
            vertex_shader_code,
            fragment_shader_code,
        }
    }

}

impl VulkanPipeline {
    pub fn new(device: &Device, render_pass: &RenderPass, desc: PipelineDesc) -> VulkanPipeline {
        let g = range_event_start!("Create pipeline");
        // no descriptor sets
        let pipeline_layout_info = PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None).unwrap() };

        // shaders
        let vert_code = desc.vertex_shader_code;
        let vert_code: Vec<u32> = vert_code.chunks(4).map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap())).collect();
        let vertex_module = unsafe { device.create_shader_module(
            &ShaderModuleCreateInfo::default().code(&vert_code), None)
        }.unwrap();

        let frag_code = desc.fragment_shader_code;
        let frag_code: Vec<u32> = frag_code.chunks(4).map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap())).collect();
        let frag_module = unsafe { device.create_shader_module(
            &ShaderModuleCreateInfo::default().code(&frag_code), None)
        }.unwrap();

        let main_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let vert_stage = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::VERTEX)
            .module(vertex_module)
            .name(main_name);
        let frag_stage = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(main_name);

        // pipeline parts
        let multisample_state = PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(SampleCountFlags::TYPE_1);
        let dynamic_state = PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]);

        let vertex_input = PipelineVertexInputStateCreateInfo::default();
        let input_assembly = PipelineInputAssemblyStateCreateInfo::default()
            .topology(PrimitiveTopology::TRIANGLE_LIST);

        let rast_info = PipelineRasterizationStateCreateInfo::default()
            .cull_mode(CullModeFlags::NONE)
            .line_width(1.0);

        let viewport_state = PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let color_blend_attachment = [PipelineColorBlendAttachmentState::default().color_write_mask(ColorComponentFlags::RGBA)];
        let color_blend = PipelineColorBlendStateCreateInfo::default()
            .attachments(&color_blend_attachment);

        let depth_state = PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(CompareOp::LESS);


        let stages = [vert_stage, frag_stage];
        let pipeline_create_info = GraphicsPipelineCreateInfo::default()
            .layout(pipeline_layout)
            .render_pass(*render_pass)
            .dynamic_state(&dynamic_state)
            .multisample_state(&multisample_state)

            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .stages(&stages)
            .rasterization_state(&rast_info)
            .color_blend_state(&color_blend)
            .viewport_state(&viewport_state)
            .depth_stencil_state(&depth_state);

        let pipeline_cache = unsafe {
            device.create_pipeline_cache(&PipelineCacheCreateInfo::default(), None).unwrap()
        };

        let pipeline = unsafe { device.create_graphics_pipelines(pipeline_cache, &[pipeline_create_info], None).unwrap()[0] };

        //destroy shader modules
        unsafe { device.destroy_shader_module(vertex_module, None); }
        unsafe { device.destroy_shader_module(frag_module, None); }

        VulkanPipeline {
            pipeline,
            pipeline_layout,
            pipeline_cache
        }
    }

    pub fn get_pipeline(&self) -> Pipeline {
        self.pipeline
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_pipeline_cache(self.pipeline_cache, None);
        device.destroy_pipeline(self.pipeline, None);
    }
}