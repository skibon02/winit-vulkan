use ash::vk;
use ash::vk::{CommandBuffer, PipelineStageFlags, QueryPool, QueryPoolCreateInfo, QueryResultFlags};
use crate::vulkan_backend::wrappers::device::{VkDevice, VkDeviceRef};

pub struct TimestampPool {
    device: VkDeviceRef,
    query_pool: QueryPool,
    slot_count: usize,
    tm_period: f32,
}

impl TimestampPool {
    pub fn new(device: VkDeviceRef, max_timestamp_slots: u32, tm_period: f32) -> Option<TimestampPool> {

        let info = QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(10);
        
        let query_pool = unsafe { device.create_query_pool(&info, None) }.ok()?;
        Some(Self {
            device: device.clone(),
            query_pool,
            slot_count: max_timestamp_slots as usize,
            tm_period
        })
    }
    pub fn write_start_timestamp(&mut self, cb: CommandBuffer, slot: u32) {
        unsafe { self.device.cmd_write_timestamp(cb, PipelineStageFlags::TOP_OF_PIPE, self.query_pool, slot); }
    }
    pub fn write_end_timestamp(&mut self, cb: CommandBuffer, slot: u32) {
        unsafe { self.device.cmd_write_timestamp(cb, PipelineStageFlags::BOTTOM_OF_PIPE, self.query_pool, slot); }
    }
    
    pub fn reset_all_slots(&mut self) {
        unsafe { self.device.reset_query_pool(self.query_pool, 0, self.slot_count as u32); }
    }
    
    pub fn cmd_reset(&mut self, cb: CommandBuffer) {
        unsafe { self.device.cmd_reset_query_pool(cb,  self.query_pool, 0, self.slot_count as u32) };
    }
    
    pub fn read_timestamps(&mut self, start_slot: u32) -> Option<(u64, u64)> {
        let mut timestamps = [0u64; 2];
        unsafe { self.device.get_query_pool_results(self.query_pool, start_slot, &mut timestamps, QueryResultFlags::TYPE_64).ok()? };
        Some((timestamps[0], timestamps[1]))
    }
    
}

impl Drop for TimestampPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_query_pool(self.query_pool, None); }
    }
}
