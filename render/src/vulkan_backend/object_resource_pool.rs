use std::any::TypeId;
use std::collections::{btree_map, BTreeMap};
use std::collections::btree_map::Entry;
use std::path::Path;
use ash::vk;
use ash::vk::{BufferUsageFlags, DeviceSize, Extent2D, ImageLayout, ImageTiling, ImageView, PipelineBindPoint, PrimitiveTopology, SampleCountFlags};
use log::info;
use smallvec::SmallVec;
use render_core::collect_state::{CollectDrawStateUpdates, GraphicsUpdateCmd};
use render_core::collect_state::buffer_updates::BufferUpdateData;
use render_core::object_handles::{ObjectId, UniformResourceId};
use render_core::{BufferUpdateCmd, ObjectUpdate2DCmd, UniformBufferCmd};
use render_core::collect_state::uniform_updates::ImageCmd;
use crate::util::get_resource;
use crate::util::image::read_image_from_bytes;
use crate::vulkan_backend::descriptor_sets::{DescriptorSetPool, ObjectDescriptorSet};
use crate::vulkan_backend::pipeline::{VulkanPipeline};
use crate::vulkan_backend::render_pass::RenderPassWrapper;
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use crate::vulkan_backend::wrappers::image::imageview_info_for_image;

/// Represented by a single instance attrib buffer and fixed draw count number
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
        let image = resource_manager.create_fill_image(extent, vk::Format::R8G8B8A8_UNORM, ImageTiling::OPTIMAL,
                                                  vk::ImageUsageFlags::SAMPLED, SampleCountFlags::TYPE_1, image_data.as_slice(), ImageLayout::SHADER_READ_ONLY_OPTIMAL);

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

    pub fn update_objects(&mut self, resource_manager: &mut ResourceManager,
                              draw_state_updates: &mut impl CollectDrawStateUpdates,
                              render_pass: &RenderPassWrapper) {
        // 1) perform updates
        let updates_iter = draw_state_updates.collect_updates();
        for update_cmd in updates_iter {
            match update_cmd {
                GraphicsUpdateCmd::Object2D(id, object_cmd) => match object_cmd {
                    ObjectUpdate2DCmd::Create {
                        pipeline_desc,
                        uniform_bindings_desc: uniform_bindings,
                        initial_state
                    } => {
                        let entry = self.objects.entry(id);
                        let Entry::Vacant(entry) = entry else {
                            panic!("Renderer update: object already exists");
                        };
                        assert_eq!(initial_state.buffer_offset, 0);
                        entry.insert({
                            info!("Creating new object with id: {}", id);
                            let pipeline_desc = pipeline_desc();
                            let pipeline_entry = self.pipelines.entry(pipeline_desc.id).or_insert_with(|| {
                                info!("Creating new pipeline with id: {:?}", pipeline_desc.id);

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
                                                                          uniform_bindings.buffer_bindings.iter().map(|(binding, buffer_id)| {
                                                                              (*binding, *self.uniform_buffers.get(buffer_id).unwrap())
                                                                          }),
                                                                          uniform_bindings.image_bindings.iter().map(|(binding, image_id)| {
                                                                              (*binding, self.image_resources.get(image_id).unwrap())
                                                                          }));

                            // create vertex buffer for per-instance attributes
                            let vertex_data = initial_state.modified_bytes;
                            let vertex_buffer_per_ins = resource_manager.create_fill_buffer(
                                vertex_data.len() as DeviceSize,
                                BufferUsageFlags::VERTEX_BUFFER,
                                vertex_data
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
                    }
                    ObjectUpdate2DCmd::AttribUpdate(buffer_update) => match buffer_update {
                        BufferUpdateCmd::Update(BufferUpdateData { modified_bytes, buffer_offset }) => {
                            // info!("Updating object with id: {}.", id);
                            let entry = self.objects.get_mut(&id).expect("Renderer update: object does not exist");
                            resource_manager.fill_buffer(entry.vertex_buffer_per_ins, &modified_bytes, buffer_offset);
                        }
                        _ => {
                            unimplemented!("Renderer update: object attrib update is not implemented");
                        }
                    }
                    ObjectUpdate2DCmd::Destroy => {
                        let entry = self.objects.remove(&id).expect("Renderer update: object does not exist");
                        info!("Destroying object with id: {}", id);
                        
                        // destroy DescriptorSet
                        let descriptor_pool = &mut self.descriptor_set_pool;
                        entry.descriptor_set.destroy(descriptor_pool);
                        
                        // destroy attrib buffer
                        resource_manager.destroy_buffer(entry.vertex_buffer_per_ins);
                    }
                }
                GraphicsUpdateCmd::UniformBuffer(id, uniform_cmd) => match uniform_cmd {
                    UniformBufferCmd::Create(BufferUpdateData { modified_bytes, buffer_offset }) => {
                        let entry = self.uniform_buffers.entry(id);
                        assert_eq!(buffer_offset, 0);
                        let Entry::Vacant(entry) = entry else {
                            panic!("Renderer update: uniform buffer already exists");
                        };
                        entry.insert({
                            info!("Creating new uniform buffer with id: {}", id);
                            let buffer = resource_manager.create_fill_buffer(
                                modified_bytes.len() as DeviceSize,
                                BufferUsageFlags::UNIFORM_BUFFER,
                                modified_bytes
                            );
                            buffer
                        });
                    }
                    UniformBufferCmd::Update(buffer_update) => match buffer_update {
                        BufferUpdateCmd::Update(BufferUpdateData { modified_bytes, buffer_offset }) => {
                            // info!("Updating uniform buffer with id: {}.", id);
                            let entry = self.uniform_buffers.get(&id).expect("Renderer update: uniform buffer does not exist");
                            resource_manager.fill_buffer(*entry, &modified_bytes, buffer_offset);
                        }
                        BufferUpdateCmd::Resize(new_size) => {
                            unimplemented!("Renderer update: uniform buffer resize is not implemented");
                        }
                        BufferUpdateCmd::Rearrange(copy_ops) => {
                            unimplemented!("Renderer update: uniform buffer rearrange is not implemented");
                        }
                    }
                    UniformBufferCmd::Destroy => {
                        unimplemented!("Renderer update: uniform buffer destroy is not implemented");
                    }
                }
                GraphicsUpdateCmd::Image(id, image_cmd) => match image_cmd {
                    ImageCmd::Create(path) => {
                        let entry = self.image_resources.entry(id);
                        let Entry::Vacant(entry) = entry else {
                            panic!("Renderer update: image resource already exists");
                        };
                        entry.insert({
                            info!("Creating new image resource with id: {}", id);
                            let data = get_resource(Path::join("resources".as_ref(), path)).unwrap();
                            let (image_data, extent) = read_image_from_bytes(data).unwrap();
                            info!("Image extent: {:?}", extent);
                            UniformImage::new(image_data, extent, resource_manager, self.device.clone())
                        });
                    }
                    ImageCmd::Destroy => {
                        unimplemented!("Renderer update: uniform resource destroy is not implemented");
                    }
                }
            }
        }
        
        
        // 2) insert READ_AFTER_WRITE barrier for transfer and host write operations
        let cb = resource_manager.take_commands();
        let command_buffers = [cb];
        unsafe {
            let memory_barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE | vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)];
            self.device.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TRANSFER | vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::ALL_GRAPHICS,
                vk::DependencyFlags::empty(),
                &memory_barrier,
                &[],
                &[],
            );
            self.device.end_command_buffer(cb).unwrap();
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers);
            self.device.queue_submit(resource_manager.queue(), &[submit_info], vk::Fence::null()).unwrap();
        }
    }

    pub fn record_draw_commands(&mut self, command_buffer: vk::CommandBuffer) {
        for (id, draw_state) in self.objects.iter_mut().rev() {
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