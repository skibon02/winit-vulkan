use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use crate::vulkan_backend::wrappers::command_pool::{CommandBufferPair, VkCommandPool};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use crate::vulkan_backend::wrappers::image::{image_2d_info, get_aspect_mask};
use ash::vk::{self, Buffer, BufferCreateFlags, BufferUsageFlags, CommandBuffer, CommandBufferUsageFlags, DeviceMemory, DeviceSize, Extent2D, Extent3D, Format, ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageTiling, ImageUsageFlags, MappedMemoryRange, MemoryPropertyFlags, SampleCountFlags, Sampler};
use std::fmt::Debug;
use std::ops::Range;
use log::info;
use sparkles::range_event_start;

const STAGING_BUFFER_SIZE: usize = 1024 * 1024 * 128; // 128 MB

/// User is responsible for not using this buffer after it's destroyed
#[derive(Clone, Copy)]
pub struct BufferResource {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

/// User is responsible for not using this image after it's destroyed
#[derive(Clone, Copy)]
pub struct ImageResource {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
    pub info: ImageCreateInfo<'static>,

    extent: Extent3D,
}

/// Safety:
/// 1) data.len() * size_of::<T>() must be <= `size`
/// 2) offset..offset+size range of `memory` must not be used by any commands which are not completed
/// 3) `memory` must be allocated from HOST_COHERENT memory type
unsafe fn map_and_write_memory<T: Copy>(device: &VkDeviceRef, memory: vk::DeviceMemory, offset: DeviceSize, size: DeviceSize, data: &[T]) {
    let mem_ptr = device
        .map_memory(memory, offset, size, vk::MemoryMapFlags::empty())
        .unwrap();
    assert_eq!(mem_ptr.align_offset(std::mem::align_of::<T>()), 0, "Memory is not properly aligned");

    let mem_slice = std::slice::from_raw_parts_mut(mem_ptr as *mut T, data.len());
    mem_slice.copy_from_slice(data);
    device.unmap_memory(memory);
}

/// Safety:
/// 1) data.len() * size_of::<T>() must be <= `size`
/// 2) offset..offset+size range of `memory` must not be used by any commands which are not completed
/// 3) `memory` must be allocated from HOST_VISIBLE memory type
unsafe fn map_and_write_memory_non_coherent<T: Copy>(device: &VkDeviceRef, memory: vk::DeviceMemory, offset: DeviceSize, size: DeviceSize, data: &[T]) {
    let mem_ptr = device
        .map_memory(memory, offset, size, vk::MemoryMapFlags::empty())
        .unwrap();
    assert_eq!(mem_ptr.align_offset(std::mem::align_of::<T>()), 0, "Memory is not properly aligned");

    device.invalidate_mapped_memory_ranges(&[MappedMemoryRange::default().memory(memory).offset(offset).size(size)]).unwrap();
    let mem_slice = std::slice::from_raw_parts_mut(mem_ptr as *mut T, data.len());
    mem_slice.copy_from_slice(data);
    device.flush_mapped_memory_ranges(&[MappedMemoryRange::default().memory(memory).offset(offset).size(size)]).unwrap();
    device.unmap_memory(memory);
}

pub struct StagingManager {
    // Strategy for staging buffer: use multiple ranges of a single big buffer
    staging_buf_memory: vk::DeviceMemory,
    staging_buf: Buffer,
    staging_buf_allocations: BTreeSet<BufferRange>,

    device: VkDeviceRef,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferRange(Range<DeviceSize>);

impl PartialOrd<Self> for BufferRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferRange {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.start.cmp(&other.0.start)
    }
}

impl StagingManager {
    pub fn new(device: VkDeviceRef, memory_type: u32) -> Self {
        let staging_buf_create_info = vk::BufferCreateInfo::default()
            .size(STAGING_BUFFER_SIZE as u64)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buf = unsafe { device.create_buffer(&staging_buf_create_info, None) }.unwrap();

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(staging_buf) };

        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type);

        let staging_buf_memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.unwrap();

        unsafe { device.bind_buffer_memory(staging_buf, staging_buf_memory, 0) }.unwrap();

        Self {
            staging_buf_memory,
            staging_buf,
            staging_buf_allocations: BTreeSet::new(),

            device
        }
    }

    /// Try to find a range of size `size` in the staging buffer and return it
    ///
    /// Panic if there is no space left in the staging buffer
    fn allocate(&mut self, size: DeviceSize) -> Range<DeviceSize> {
        let mut end = 0;
        for range in self.staging_buf_allocations.iter() {
            if range.0.start - end >= size {
                let new_range = BufferRange(end..end + size);
                self.staging_buf_allocations.insert(new_range.clone());
                return new_range.0.clone();
            }
            end = range.0.end;
        }

        if STAGING_BUFFER_SIZE as DeviceSize - end >= size {
            let new_range = BufferRange(end..end + size);
            self.staging_buf_allocations.insert(new_range.clone());
            return new_range.0.clone();
        }

        panic!("Attempt to allocate {} bytes in staging buffer failed!", size);
    }

    /// Write data to the staging buffer and return the range
    ///
    /// Safety:
    /// 1) data.len() * size_of::<T>() must be less than `size`
    pub fn allocate_and_write<T: Copy>(&mut self, size: DeviceSize, data: &[T]) -> Range<DeviceSize> {
        assert!(data.len() as DeviceSize * size_of::<T>() as DeviceSize <= size);
        let range = self.allocate(size);
        unsafe {
            map_and_write_memory(&self.device, self.staging_buf_memory, range.start, size, data);
        }

        range
    }

    /// Record a command to transfer data from the staging buffer to the given buffer range
    ///
    /// Safety:
    /// 1) `staging_range` must be result of previously executed function `allocate_and_write`.
    /// 2) `free_allocations` must not be called between call to `allocate_and_write` and `transfer_to_buf`
    pub fn transfer_to_buf(&mut self, staging_range: Range<DeviceSize>, command_buffer: vk::CommandBuffer, buffer: Buffer, buffer_offset: DeviceSize) {
        let copy_region = vk::BufferCopy::default()
            .size(staging_range.end - staging_range.start)
            .src_offset(staging_range.start)
            .dst_offset(buffer_offset);

        unsafe {
            self.device.cmd_copy_buffer(command_buffer, self.staging_buf, buffer, &[copy_region]);
        }
    }

    /// Record a command to transfer data from the staging buffer to the given image
    ///
    /// Safety:
    /// 1) `staging_range` must be result of previously executed function `allocate_and_write`.
    /// 2) `free_allocations` must not be called between call to `allocate_and_write` and `transfer_to_img`
    /// 3) Image must be in layout `TRANSFER_DST_OPTIMAL`
    pub fn transfer_to_img(&mut self, staging_range: Range<DeviceSize>, command_buffer: vk::CommandBuffer, image: ImageResource, aspect: ImageAspectFlags) {
        let copy_region = vk::BufferImageCopy::default()
            .buffer_offset(staging_range.start)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(aspect)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(image.extent);

        unsafe {
            self.device.cmd_copy_buffer_to_image(command_buffer, self.staging_buf, image.image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[copy_region]);
        }
    }

    pub fn free_allocations(&mut self) {
        self.staging_buf_allocations.clear();
    }

}

impl Drop for StagingManager {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.staging_buf_memory, None);
            self.device.destroy_buffer(self.staging_buf, None);
        }
    }
}

pub struct ResourceManager {
    image_resources: Vec<ImageResource>,
    buffer_resources: Vec<BufferResource>,
    sampler_resources: Vec<Sampler>,

    device: VkDeviceRef,
    queue: vk::Queue,
    command_buffer: CommandBufferPair,

    memory_types: Vec<vk::MemoryType>,

    staging_manager: StagingManager,

    device_local_buffer_memory_type: BTreeMap<(vk::BufferUsageFlags, vk::BufferCreateFlags), u32>,
    // Key: tiling, is_color_format
    device_local_image_memory_type: BTreeMap<(ImageTiling, bool), u32>,
}

impl ResourceManager {
    pub fn new(
        physical_device: vk::PhysicalDevice,
        device: VkDeviceRef,
        queue: vk::Queue,
        command_pool: &VkCommandPool,
    ) -> Self {
        // allocate command buffer
        let command_buffer = CommandBufferPair::new(command_pool.alloc_command_buffers(2).try_into().unwrap(), &device);

        //query memory properties info
        let memory_properties = unsafe {
            device
                .instance()
                .get_physical_device_memory_properties(physical_device)
        };

        let host_memory_type= memory_properties
            .memory_types_as_slice()
            .iter()
            .position(|memory_type| {
                memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT) // host visible and coherent
            }).expect("Having at least one memory type with HOST_COHERENT is guaranteed by the spec!") as u32;

        let staging_manager = StagingManager::new(device.clone(), host_memory_type);

        Self {
            buffer_resources: Vec::new(),
            image_resources: Vec::new(),
            sampler_resources: Vec::new(),

            device,
            queue,
            command_buffer,

            memory_types: memory_properties.memory_types_as_slice().to_vec(),
            staging_manager,

            device_local_buffer_memory_type: BTreeMap::new(),
            device_local_image_memory_type: BTreeMap::new(),
        }
    }

    pub fn queue(&self) -> vk::Queue {
        self.queue
    }

    /// Find memory type index for buffer with given usage and flags
    fn get_buffer_memory_requirements(&mut self, usage: BufferUsageFlags, flags: BufferCreateFlags) -> u32 {
        let device_memory_type = self.device_local_buffer_memory_type
            .entry((usage, flags))
            .or_insert_with(|| {
                // create a dummy buffer to get memory requirements
                let buffer_create_info = vk::BufferCreateInfo::default()
                    .size(1)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE);

                let buffer = unsafe { self.device.create_buffer(&buffer_create_info, None) }.unwrap();
                let memory_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
                unsafe { self.device.destroy_buffer(buffer, None) };

                let memory_type = self.memory_types
                    .iter()
                    .enumerate()
                    .max_by_key(|(i, memory_type)| {
                        let mut r = 0;
                        if memory_requirements.memory_type_bits & (1 << i) != 0 {
                            r += 100;
                        }
                        if memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
                            r += 10;
                        }
                        if memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT) {
                            r += 1;
                        }
                        if memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
                            r += 1;
                        }
                        r
                    })
                    .unwrap();

                memory_type.0 as u32
            });

        *device_memory_type
    }

    fn get_image_memory_requirements(&mut self, tiling: ImageTiling, format: Format) -> u32 {
        let is_color_format = get_aspect_mask(format).contains(ImageAspectFlags::COLOR);
        let device_memory_type = self.device_local_image_memory_type
            .entry((tiling, is_color_format))
            .or_insert_with(|| {
                // create a dummy image to get memory requirements
                let image_create_info = vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D {
                        width: 1,
                        height: 1,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::TRANSFER_DST)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED);

                let image = unsafe { self.device.create_image(&image_create_info, None) }.unwrap();
                let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };
                unsafe { self.device.destroy_image(image, None) };

                self.memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        memory_requirements.memory_type_bits & (1 << i) != 0
                            && memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                    })
                    .unwrap() as u32
            });

        *device_memory_type
    }

    /// Create a new buffer with the given size and usage flags
    ///
    /// The buffer will be allocated in memory with type DEVICE_LOCAL.
    pub fn create_buffer(
        &mut self,
        size: vk::DeviceSize,
        mut usage: vk::BufferUsageFlags,
    ) -> BufferResource {
        let flags = BufferCreateFlags::empty();
        let device_memory_type = self.get_buffer_memory_requirements(usage | BufferUsageFlags::TRANSFER_DST, flags);
        if !self.memory_types[device_memory_type as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            usage |= vk::BufferUsageFlags::TRANSFER_DST;
        }

        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { self.device.create_buffer(&buffer_create_info, None) }.unwrap();


        let memory_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(device_memory_type);
        let memory = unsafe { self.device.allocate_memory(&memory_allocate_info, None) }.unwrap();

        unsafe { self.device.bind_buffer_memory(buffer, memory, 0) }.unwrap();

        let res = BufferResource {
            buffer,
            memory,
            size,
        };
        self.buffer_resources.push(res);

        res
    }

    /// Same as `create_buffer`, but also fills the buffer with the provided initial data
    ///
    /// Filling can be done without staging buffer if device memory type is HOST_VISIBLE
    ///
    /// You need to explicitly submit transfer operation by calling `submit_transfer` method
    ///
    /// Safety:
    /// 1) data.len() * size_of::<T>() must be less than `size`
    pub fn create_fill_buffer<T: Copy>(&mut self, size: vk::DeviceSize, usage: vk::BufferUsageFlags, data: &[T]) -> BufferResource {
        assert!(size_of_val(data) as vk::DeviceSize <= size);

        let buffer = self.create_buffer(size, usage);

        let device_memory_type = self.get_buffer_memory_requirements(usage | BufferUsageFlags::TRANSFER_DST, BufferCreateFlags::empty());
        let memory_type = &self.memory_types[device_memory_type as usize];
        // we can fill buffer immediately without staging buffer if device memory type is HOST_VISIBLE
        if memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            if memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT) {
                unsafe {map_and_write_memory(&self.device, buffer.memory, 0, size, data)};
            }
            else {
                unsafe {map_and_write_memory_non_coherent(&self.device, buffer.memory, 0, size, data)};
            }
        }
        else {
            // 1) Write data to staging
            let staging_range = self.staging_manager.allocate_and_write(size, data);

            // 2) Prepare transfer command
            let cb = self.command_buffer.current_cb();
            self.staging_manager.transfer_to_buf(
                staging_range,
                cb,
                buffer.buffer,
                0,
            );

        }

        buffer
    }

    /// Fill the buffer assuming it is in use.
    /// All previous and future accesses must be synchronized externally.
    ///
    /// You need to explicitly submit transfer operation by calling `submit_transfer` method
    pub fn fill_buffer<T: Copy + Debug>(&mut self, resource: BufferResource, data: &[T], offset: usize) {
        //size check
        let size = size_of_val(data) as vk::DeviceSize;
        assert!(size <= resource.size);


        // 1) write to staging buffer
        let staging_range = self.staging_manager.allocate_and_write(size, data);

        // 2) transfer staging -> device_local
        let cb = self.command_buffer.current_cb();
        self.staging_manager.transfer_to_buf(
            staging_range,
            cb,
            resource.buffer,
            offset as vk::DeviceSize,
        );
    }

    pub fn destroy_buffer(&mut self, buffer: BufferResource) {
        if let Some(index) = self
            .buffer_resources
            .iter()
            .position(|resource| resource.memory == buffer.memory)
        {
            self.buffer_resources.swap_remove(index);
        }

        unsafe {
            self.device.free_memory(buffer.memory, None);
            self.device.destroy_buffer(buffer.buffer, None);
        }
    }

    /// Submit all prepared transfer commands on the queue
    ///
    /// This function must be explicitly called after all methods that transfer data from host to buffers
    pub fn take_commands(&mut self) -> CommandBuffer {
        self.command_buffer.swap_buffers(&self.device)
    }

    /// Create a new image with the given extent, format, tiling, usage flags and sample count
    ///
    /// The image will be allocated in memory with type DEVICE_LOCAL.
    pub fn create_image(
        &mut self,
        extent: Extent2D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        mut usage: vk::ImageUsageFlags,
        sample_count: SampleCountFlags,
    ) -> ImageResource {
        let extent = Extent3D::from(extent);

        let device_memory_type = self.get_image_memory_requirements(tiling, format);
        if !self.memory_types[device_memory_type as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            usage |= vk::ImageUsageFlags::TRANSFER_DST;
        }

        let image_create_info = image_2d_info(format, usage, extent, sample_count, tiling);
        let image = unsafe { self.device.create_image(&image_create_info, None) }.unwrap();

        let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };

        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(device_memory_type);

        let memory = unsafe { self.device.allocate_memory(&memory_allocate_info, None) }.unwrap();

        unsafe { self.device.bind_image_memory(image, memory, 0) }.unwrap();
        
        // transition to TRANSFER_DST_OPTIMAL layout
        let cb = self.command_buffer.current_cb();
        let image_memory_barrier = [ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::NONE)
            .dst_access_mask(vk::AccessFlags::NONE)
            .old_layout(ImageLayout::UNDEFINED)
            .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(get_aspect_mask(format))
                .layer_count(1)
                .level_count(1)
            )
        ];
        unsafe {
            self.device.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_memory_barrier,
            );
        }

        let res = ImageResource {
            image,
            memory,
            size: memory_requirements.size,
            extent,
            info: image_create_info,
        };
        
        self.image_resources.push(res);
        
        res
    }

    /// Same as `create_image`, but also fills the image with the provided initial data
    ///
    /// Filling can be done without staging buffer if device memory type is HOST_VISIBLE
    /// An automatic synchronization is performed with reads in FRAGMENT_SHADER pipeline stage,
    ///
    /// You need to explicitly submit transfer operation by calling `submit_transfer` method
    ///
    /// Safety:
    /// 1) data.len() must be <= image size
    /// 2) format must be color format (todo: we can support depth/stencil too)
    pub fn create_fill_image(&mut self, extent: Extent2D, format: vk::Format, tiling: vk::ImageTiling, usage: vk::ImageUsageFlags, sample_count: SampleCountFlags, data: &[u8], final_layout: ImageLayout) -> ImageResource {
        let image = self.create_image(extent, format, tiling, usage | ImageUsageFlags::TRANSFER_DST, sample_count);
        assert!(data.len() <= image.size as usize);

        // 1) Write data to staging
        let staging_range = self.staging_manager.allocate_and_write(data.len() as vk::DeviceSize, data);

        // 2) Prepare transfer command
        let cb = self.command_buffer.current_cb();
        self.staging_manager.transfer_to_img(
            staging_range,
            cb,
            image,
            ImageAspectFlags::COLOR,
        );

        // 3) Transition image layout
        let cb = self.command_buffer.current_cb();
        let image_memory_barrier = [ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::NONE)
            .dst_access_mask(vk::AccessFlags::NONE)
            .old_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(final_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image.image)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .layer_count(1)
                .level_count(1)
            )
        ];
        unsafe {
            self.device.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_memory_barrier,
            );
        }

        image
    }

    // /// Fill the image assuming it is in use.
    // /// All previous and future accesses must be synchronized externally.
    // ///
    // /// You need to explicitly submit transfer operation by calling `submit_transfer` method
    // ///
    // /// Safety:
    // /// 1) data.len() must be <= image size
    // pub fn fill_image(&mut self, image_resource: ImageResource, data: &[u8]) {
    //     //size check
    //     let size = data.len() as vk::DeviceSize;
    //     assert!(size <= image_resource.size);
    //
    //     // 1) write to staging buffer
    //     let staging_range = self.staging_manager.allocate_and_write(size, data);
    //
    //     // 2) transfer staging -> device_local
    //     let cb = self.command_buffer.command_buffer_for_writing(&self.device);
    //     self.staging_manager.transfer_to_img(
    //         staging_range,
    //         cb,
    //         image_resource,
    //         ImageAspectFlags::COLOR,
    //     );
    // }

    pub fn destroy_image(&mut self, image: ImageResource) {
        if let Some(index) = self
            .image_resources
            .iter()
            .position(|resource| resource.memory == image.memory)
        {
            self.image_resources.swap_remove(index);
        }

        unsafe {
            self.device.free_memory(image.memory, None);
        }
        unsafe { self.device.destroy_image(image.image, None) };
    }

    pub fn create_sampler(&mut self) -> Sampler {
        let sampler_create_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(0.0)
            .mip_lod_bias(0.0);

        let sampler = unsafe { self.device.create_sampler(&sampler_create_info, None) }.unwrap();
        self.sampler_resources.push(sampler);

        sampler
    }
    
    pub fn free_staging_allocations(&mut self) {
        self.staging_manager.free_allocations();
    }
}
impl Drop for ResourceManager {
    fn drop(&mut self) {
        let g = range_event_start!("[Vulkan] Destroy resource manager");
        for image_res in self.image_resources.drain(..) {
            unsafe {
                self.device.free_memory(image_res.memory, None);
                self.device.destroy_image(image_res.image, None);
            }
        }

        for buffer_res in self.buffer_resources.drain(..) {
            unsafe {
                self.device.free_memory(buffer_res.memory, None);
                self.device.destroy_buffer(buffer_res.buffer, None);
            }
        }
        for sampler_res in self.sampler_resources.drain(..) {
            unsafe {
                self.device.destroy_sampler(sampler_res, None);
            }
        }
    }
}
