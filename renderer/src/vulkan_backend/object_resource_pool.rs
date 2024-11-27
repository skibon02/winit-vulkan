use std::any::TypeId;
use std::collections::BTreeMap;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, PipelineBindPoint, PrimitiveTopology};
use log::info;
use smallvec::SmallVec;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::PipelineDescWrapper;
use crate::state::{DrawStateCollect, ObjectStateWrapper};
use crate::use_shader;
use crate::vulkan_backend::descriptor_sets::{DescriptorSetPool, ObjectDescriptorSet};
use crate::vulkan_backend::pipeline::{VertexInputDesc, VulkanPipeline};
use crate::vulkan_backend::render_pass::RenderPassWrapper;
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;

pub struct ObjectDrawState {
    vertex_buffer_per_ins: BufferResource,
    vertex_count: usize,
    instance_count: usize,
    descriptor_set: ObjectDescriptorSet,
    pipeline_id: TypeId,
}

pub struct ObjectResourcePool {
    device: VkDeviceRef,
    descriptor_set_pool: DescriptorSetPool,

    pipelines: BTreeMap<TypeId, VulkanPipeline>,
    objects: BTreeMap<ObjectId, ObjectDrawState>,
    uniform_buffers: BTreeMap<UniformResourceId, BufferResource>
}

impl ObjectResourcePool {
    pub fn new(device: VkDeviceRef) -> Self {
        let descriptor_set_pool = DescriptorSetPool::new(device.clone());
        ObjectResourcePool {
            device,
            descriptor_set_pool,

            objects: BTreeMap::new(),
            pipelines: BTreeMap::new(),
            uniform_buffers: BTreeMap::new(),
        }
    }

    pub fn update_objects<'a>(&mut self, resource_manager: &mut ResourceManager,
                  draw_state_updates: &mut impl DrawStateCollect,
                  render_pass: &RenderPassWrapper) {
        let uniform_updates = draw_state_updates.collect_uniform_updates();
        for (id, uniform_data, uniform_offset) in uniform_updates {
            let entry = self.uniform_buffers.entry(id).or_insert_with(|| {
                info!("Creating new uniform buffer with id: {}", id);
                let buffer = resource_manager.create_buffer(
                    uniform_data.len() as DeviceSize,
                    BufferUsageFlags::UNIFORM_BUFFER,
                );
                buffer
            });
            info!("Updating uniform buffer with id: {}. Data: {:?}", id, uniform_data);
            resource_manager.fill_buffer(*entry, &uniform_data, uniform_offset);
        }

        let objects_updates = draw_state_updates.collect_object_updates();
        for (id, obj_state, pipeline_desc) in objects_updates {
            let entry = self.objects.entry(id).or_insert_with(|| {
                info!("Creating new object with id: {}", id);
                let pipeline_desc = pipeline_desc();
                let pipeline_entry = self.pipelines.entry(pipeline_desc.id).or_insert_with(|| {
                    info!("Creating new pipeline with id: {:?}, Desc: {:?}", pipeline_desc.id, &pipeline_desc);

                    let pipeline_desc = pipeline_desc.clone();
                    let pipeline = VulkanPipeline::new(
                        self.device.clone(),
                        render_pass,
                        pipeline_desc,
                    );
                    pipeline
                });

                let descriptor_set = ObjectDescriptorSet::new(self.device.clone(),
                   &mut self.descriptor_set_pool, pipeline_entry.get_descriptor_set_layout(),
                    obj_state.uniform_bindings.clone().into_iter().map(|(binding, buffer_id)| {
                        (binding, *self.uniform_buffers.get(&buffer_id).unwrap())
                    }));

                // create vertex buffer for per-instance attributes
                let vertex_data = obj_state.attributes_data;
                let vertex_buffer_per_ins = resource_manager.create_buffer(
                    vertex_data.len() as DeviceSize,
                    BufferUsageFlags::VERTEX_BUFFER,
                );

                // for now, it is 1
                let instance_count = 1;
                
                ObjectDrawState {
                    vertex_buffer_per_ins,
                    vertex_count: instance_count * pipeline_desc.vertices_per_instance,
                    instance_count,
                    descriptor_set,
                    pipeline_id: pipeline_desc.id,
                }
            });
            info!("Updating object with id: {}. State: {:?}", id, obj_state);

            // update per-instance attributes
            let vertex_data = obj_state.attributes_data;
            resource_manager.fill_buffer(entry.vertex_buffer_per_ins, &vertex_data, obj_state.data_offset);
        }

        // match state {
        //     DrawStateDiff::Create => {
        //         let object_entry = self.objects.entry(0);
        //          let descriptor_sets = ObjectDescriptorSet::new(self.device.clone(), resource_manager, &mut self.descriptor_set_pool);
        //
        //         let total_floats_per_attrib = vert_desc.get_floats_for_binding(0);
        //
        //         let descriptor_sets = ObjectDescriptorSet::new(self.device.clone(), resource_manager, &mut self.descriptor_set_pool);
        //         let pipeline = VulkanPipeline::new(
        //             self.device.clone(),
        //             render_pass,
        //             pipeline_desc,
        //             vert_desc,
        //             descriptor_sets.get_descriptor_set_layout(),
        //         );
        //
        //         let vertex_data = vec![-1.0f32, 1.0, 0.0, 1.0, 0.0, 1.0,
        //                                0.0, -1.0, 0.0, 0.0, 1.0, 1.0,
        //                                1.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        //         let vertex_buffer = resource_manager.create_buffer(
        //             (vertex_data.len() * 4) as DeviceSize,
        //             BufferUsageFlags::VERTEX_BUFFER,
        //         );
        //         let vertex_count = vertex_data.len() / total_floats_per_attrib;
        //
        //         resource_manager.fill_buffer(vertex_buffer, &vertex_data);
        //
        //         object_entry.or_insert(ObjectDrawState {
        //             vertex_buffer,
        //             vertex_count,
        //             instance_count: 1,
        //             descriptor_sets,
        //             pipeline,
        //         });
        //     }
        //     DrawStateDiff::Modify(new_color) => {
        //         let mut object_entry_ref = self.objects.first_entry().unwrap();
        //         let object_entry = object_entry_ref.get_mut();
        //         object_entry.descriptor_sets.update(resource_manager, new_color);
        //     }
        // }
    }

    pub fn record_draw_commands(&mut self, command_buffer: vk::CommandBuffer) {
        for (id, draw_state) in &mut self.objects {
            let pipeline = self.pipelines.get(&draw_state.pipeline_id).unwrap();
            unsafe {
                self.device.cmd_bind_pipeline(
                    command_buffer,
                    PipelineBindPoint::GRAPHICS,
                    pipeline.get_pipeline(),
                );
                self.device.cmd_bind_vertex_buffers(command_buffer, 0, &[draw_state.vertex_buffer_per_ins.buffer], &[0]);
                draw_state.descriptor_set.bind_sets(command_buffer, pipeline.get_pipeline_layout());
                //draw
                self.device.cmd_draw(command_buffer, draw_state.vertex_count as u32,
                                     draw_state.instance_count as u32, 0, 0);
            }
        }
    }
}