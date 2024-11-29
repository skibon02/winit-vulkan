use std::any::TypeId;
use std::collections::{btree_map, BTreeMap};
use std::collections::btree_map::Entry;
use std::path::Path;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, Extent2D, ImageTiling, ImageView, PipelineBindPoint, PrimitiveTopology, SampleCountFlags};
use log::info;
use smallvec::SmallVec;
use crate::collect_state::{CollectDrawStateUpdates, StateUpdates};
use crate::collect_state::uniform_updates::UniformBufferUpdates;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::PipelineDescWrapper;
use crate::use_shader;
use crate::util::get_resource;
use crate::util::image::read_image_from_bytes;
use crate::vulkan_backend::descriptor_sets::{DescriptorSetPool, ObjectDescriptorSet};
use crate::vulkan_backend::pipeline::{VertexInputDesc, VulkanPipeline};
use crate::vulkan_backend::render_pass::RenderPassWrapper;
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use crate::vulkan_backend::wrappers::image::imageview_info_for_image;

pub struct ObjectDrawState {
    vertex_buffer_per_ins: BufferResource,
    vertex_count: usize,
    instance_count: usize,
    descriptor_set: ObjectDescriptorSet,
    pipeline_id: TypeId,
}

pub struct UniformImage {
    pub image_view: ImageView,
    pub sampler: vk::Sampler,
    pub dev_ref: VkDeviceRef,
}
impl UniformImage {
    pub fn new(image_data: Vec<u8>, extent: Extent2D, resource_manager: &mut ResourceManager, device: VkDeviceRef) -> Self {
        let image = resource_manager.create_image(extent, vk::Format::R8G8B8A8_UNORM, ImageTiling::OPTIMAL,
                                                  vk::ImageUsageFlags::SAMPLED, SampleCountFlags::TYPE_1);

        resource_manager.fill_image(image, image_data.as_slice());

        let imageview_info = imageview_info_for_image(image.image, image.info, vk::ImageAspectFlags::COLOR);
        let imageview = unsafe { device.create_image_view(&imageview_info, None) }.unwrap();
        let sampler = resource_manager.create_sampler();

        UniformImage {
            image_view: imageview,
            sampler,
            dev_ref: device,
        }
    }
}

impl Drop for UniformImage {
    fn drop(&mut self) {
        unsafe {
            // self.dev_ref.destroy_sampler(self.sampler, None);
            self.dev_ref.destroy_image_view(self.image_view, None);
        }
    }
}

pub struct ObjectResourcePool {
    device: VkDeviceRef,
    descriptor_set_pool: DescriptorSetPool,

    pipelines: BTreeMap<TypeId, VulkanPipeline>,
    objects: BTreeMap<ObjectId, ObjectDrawState>,
    uniform_buffers: BTreeMap<UniformResourceId, BufferResource>,
    image_resources: BTreeMap<UniformResourceId, UniformImage>,
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
            image_resources: BTreeMap::new(),
        }
    }

    pub fn update_objects<'a>(&mut self, resource_manager: &mut ResourceManager,
                              draw_state_updates: &mut impl CollectDrawStateUpdates,
                              render_pass: &RenderPassWrapper) {
        let uniform_updates_iter = draw_state_updates.collect_uniform_buffer_updates();
        for (id, uniform_updates) in uniform_updates_iter {
            match uniform_updates {
                StateUpdates::New(UniformBufferUpdates { modified_bytes, buffer_offset }) =>
                {
                    let entry = self.uniform_buffers.entry(id);
                    let Entry::Vacant(entry) = entry else {
                        panic!("Renderer update: uniform buffer already exists");
                    };
                    let entry = entry.insert({
                        info!("Creating new uniform buffer with id: {}", id);
                        let buffer = resource_manager.create_buffer(
                            modified_bytes.len() as DeviceSize,
                            BufferUsageFlags::UNIFORM_BUFFER,
                        );
                        buffer
                    });
                    info!("Updating uniform buffer with id: {}", id);
                    resource_manager.fill_buffer(*entry, &modified_bytes, buffer_offset);
                }
                StateUpdates::Update(UniformBufferUpdates { modified_bytes, buffer_offset }) =>
                {
                    info!("Updating uniform buffer with id: {}.", id);
                    let entry = self.uniform_buffers.get(&id).expect("Renderer update: uniform buffer does not exist");
                    resource_manager.fill_buffer(*entry, &modified_bytes, buffer_offset);
                }
                StateUpdates::Destroy => {
                    unimplemented!("Renderer update: uniform buffer destroy is not implemented");
                }
            }
        }

        let uniform_image_updates = draw_state_updates.collect_uniform_image_updates();
        for (id, image_updates) in uniform_image_updates {
            match image_updates {
                StateUpdates::New(path) => {
                    self.image_resources.entry(id).or_insert_with(|| {
                        info!("Creating new image resource with id: {}", id);

                        let data = get_resource(Path::join("resources".as_ref(), path)).unwrap();
                        let (image_data, extent) = read_image_from_bytes(data).unwrap();
                        info!("Image extent: {:?}", extent);
                        UniformImage::new(image_data, extent, resource_manager, self.device.clone())
                    });
                }
                StateUpdates::Update(()) => {
                    unimplemented!("Renderer update: image resource updates are not implemented");
                }
                StateUpdates::Destroy => {
                    unimplemented!("Renderer update: uniform resource destroy is not implemented");
                }
            }
        }

        let objects_updates_iter = draw_state_updates.collect_object_updates();
        for (id, object_updates) in objects_updates_iter {
            match object_updates {
                StateUpdates::New((obj_state, pipeline_desc)) => {
                    let entry = self.objects.entry(id);
                    let Entry::Vacant(entry) = entry else {
                        panic!("Renderer update: object already exists");
                    };
                    let entry = entry.insert({
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
                              obj_state.buffer_bindings.iter().map(|(binding, buffer_id)| {
                                  (*binding, *self.uniform_buffers.get(buffer_id).unwrap())
                              }),
                              obj_state.image_bindings.iter().map(|(binding, image_id)| {
                                  (*binding, self.image_resources.get(image_id).unwrap())
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
                StateUpdates::Update(obj_state) => {
                    info!("Updating object with id: {}. State: {:?}", id, obj_state);

                    // update per-instance attributes
                    let entry = self.objects.get_mut(&id).expect("Renderer update: object does not exist");
                    let vertex_data = obj_state.attributes_data;
                    resource_manager.fill_buffer(entry.vertex_buffer_per_ins, &vertex_data, obj_state.data_offset);
                }
                StateUpdates::Destroy => {
                    unimplemented!("Renderer update: object destroy is not implemented");
                }
            }
        }
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