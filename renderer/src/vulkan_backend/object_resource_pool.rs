use std::collections::BTreeMap;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, PipelineBindPoint, PrimitiveTopology};
use crate::object_handles::ObjectId;
use crate::state::DrawStateDiff;
use crate::use_shader;
use crate::vulkan_backend::descriptor_sets::{DescriptorSetPool, ObjectDescriptorSet};
use crate::vulkan_backend::pipeline::{PipelineDesc, VertexInputDesc, VulkanPipeline};
use crate::vulkan_backend::render_pass::RenderPassWrapper;
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;

pub struct ObjectDrawState {
    vertex_buffer: BufferResource,
    vertex_count: usize,
    descriptor_sets: ObjectDescriptorSet,
    pipeline: VulkanPipeline,
}

pub struct ObjectResourcePool {
    device: VkDeviceRef,
    objects: BTreeMap<ObjectId, ObjectDrawState>,
    descriptor_set_pool: DescriptorSetPool
}

impl ObjectResourcePool {
    pub fn new(device: VkDeviceRef) -> Self {
        let descriptor_set_pool = DescriptorSetPool::new(device.clone());
        ObjectResourcePool {
            objects: BTreeMap::new(),
            device,
            descriptor_set_pool
        }
    }

    pub fn apply_state(&mut self, resource_manager: &mut ResourceManager, state: DrawStateDiff, render_pass: &RenderPassWrapper) {
        match state {
            DrawStateDiff::Create => {
                let object_entry = self.objects.entry(0);

                let pipeline_desc = PipelineDesc::new(use_shader!("solid"));
                let vert_desc = VertexInputDesc::new(PrimitiveTopology::TRIANGLE_LIST)
                    .attrib_3_floats() // 0: Pos 3D
                    .attrib_3_floats(); // 1: Normal 3D

                let total_floats_per_attrib = vert_desc.get_floats_for_binding(0);

                let descriptor_sets = ObjectDescriptorSet::new(self.device.clone(), resource_manager, &self.descriptor_set_pool);
                let pipeline = VulkanPipeline::new(
                    self.device.clone(),
                    render_pass,
                    pipeline_desc,
                    vert_desc,
                    descriptor_sets.get_descriptor_set_layout(),
                );

                let vertex_data = vec![-1.0f32, 1.0, 0.0, 1.0, 0.0, 1.0,
                                       0.0, -1.0, 0.0, 0.0, 1.0, 1.0,
                                       1.0, 1.0, 0.0, 1.0, 1.0, 0.0];
                let vertex_buffer = resource_manager.create_buffer(
                    (vertex_data.len() * 4) as DeviceSize,
                    BufferUsageFlags::VERTEX_BUFFER,
                );
                let vertex_count = vertex_data.len() / total_floats_per_attrib;

                resource_manager.fill_buffer(vertex_buffer, &vertex_data);

                object_entry.or_insert(ObjectDrawState {
                    vertex_buffer,
                    vertex_count,
                    descriptor_sets,
                    pipeline,
                });
            }
            DrawStateDiff::Modify(new_color) => {
                let mut object_entry_ref = self.objects.first_entry().unwrap();
                let object_entry = object_entry_ref.get_mut();
                object_entry.descriptor_sets.update(resource_manager, new_color);
            }
        }
    }

    pub fn record_draw_commands(&mut self, command_buffer: vk::CommandBuffer) {
        let object_ref = self.objects.first_entry().unwrap();
        let object = object_ref.get();
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                object.pipeline.get_pipeline(),
            );
            self.device.cmd_bind_vertex_buffers(command_buffer, 0, &[object.vertex_buffer.buffer], &[0]);
            object.descriptor_sets.bind_sets(command_buffer, object.pipeline.get_pipeline_layout());
            //draw
            self.device.cmd_draw(command_buffer, object.vertex_count as u32, 1, 0, 0);
        }
    }
}