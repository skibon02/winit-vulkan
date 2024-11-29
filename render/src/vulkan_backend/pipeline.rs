use std::ffi::CStr;
use ash::vk;
use ash::vk::{ColorComponentFlags, CompareOp, CullModeFlags, DescriptorSetLayout, DescriptorSetLayoutBinding,
              DescriptorType, DynamicState, Format, GraphicsPipelineCreateInfo, Pipeline, PipelineCache,
              PipelineCacheCreateInfo, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
              PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo,
              PipelineLayout, PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
              PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PrimitiveTopology,
              SampleCountFlags, ShaderModuleCreateInfo, ShaderStageFlags, VertexInputAttributeDescription, VertexInputBindingDescription, FALSE};
use log::info;
use smallvec::{smallvec, SmallVec};
use sparkles_macro::range_event_start;
use render_core::layout::MemberMeta;
use render_core::layout::types::GlslTypeVariant;
use render_core::pipeline::{PipelineDescWrapper, UniformBindingType, VertexAssembly};
use crate::vulkan_backend::render_pass::RenderPassWrapper;
use crate::vulkan_backend::wrappers::device::VkDeviceRef;

pub struct VulkanPipeline {
    device: VkDeviceRef,
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    pipeline_cache: PipelineCache,
    descriptor_set_layout: DescriptorSetLayout,
}

impl VulkanPipeline {
    pub fn new(device: VkDeviceRef, render_pass: &RenderPassWrapper,
               mut pipeline_desc: PipelineDescWrapper) -> VulkanPipeline {
        let g = range_event_start!("Create pipeline");

        // 1. Create layout
        let uniform_bindings_desc = pipeline_desc.uniform_bindings;

        let bindings_desc = uniform_bindings_desc.into_iter().map(|(binding, binding_type)| {
            let descriptor_type = match binding_type {
                UniformBindingType::UniformBuffer => DescriptorType::UNIFORM_BUFFER,
                UniformBindingType::CombinedImageSampler => DescriptorType::COMBINED_IMAGE_SAMPLER,
            };
            DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_count(1)
                .descriptor_type(descriptor_type)
                .stage_flags(ShaderStageFlags::FRAGMENT | ShaderStageFlags::VERTEX)
        }).collect::<Vec<_>>();
        info!("Descriptor set layout bindings: {:?}", bindings_desc);
        let descriptor_set_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings_desc);

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .unwrap()
        };

        let set_layoutst = [descriptor_set_layout];
        let pipeline_layout_info = PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layoutst);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None).unwrap() };

        // shaders
        let vert_code = pipeline_desc.vertex_shader;
        let vert_code: Vec<u32> = vert_code.chunks(4).map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap())).collect();
        let vertex_module = unsafe { device.create_shader_module(
            &ShaderModuleCreateInfo::default().code(&vert_code), None)
        }.unwrap();

        let frag_code = pipeline_desc.fragment_shader;
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
            .rasterization_samples(render_pass.get_msaa_samples().unwrap_or(SampleCountFlags::TYPE_1));
        let dynamic_state = PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]);

        let input_assembly = get_assembly_create_info(&pipeline_desc.vertex_assembly);
        let vertex_input = pipeline_desc.attributes.get_input_state_create_info();

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
            .render_pass(*render_pass.get_render_pass())
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
            device,
            
            pipeline,
            pipeline_layout,
            pipeline_cache,
            descriptor_set_layout,
        }
    }

    pub fn get_pipeline(&self) -> Pipeline {
        self.pipeline
    }

    pub fn get_pipeline_layout(&self) -> PipelineLayout {
        self.pipeline_layout
    }
    pub fn get_descriptor_set_layout(&self) -> DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

fn get_assembly_create_info(assembly: &VertexAssembly) -> PipelineInputAssemblyStateCreateInfo {
    match assembly {
        VertexAssembly::TriangleStrip => PipelineInputAssemblyStateCreateInfo {
            topology: PrimitiveTopology::TRIANGLE_STRIP,
            primitive_restart_enable: FALSE,
            ..Default::default()
        },
        VertexAssembly::TriangleList => PipelineInputAssemblyStateCreateInfo {
            topology: PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: FALSE,
            ..Default::default()
        },
    }
}

impl Drop for VulkanPipeline {
    fn drop(&mut self) {
        let g = range_event_start!("[Vulkan] Destroy pipeline");
        unsafe {
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline_cache(self.pipeline_cache, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}