pub mod single_object;
pub mod uniform_state;
mod object_group;

pub enum DrawStateDiff {
    Create,
    Modify([f32; 3])
}

pub trait DrawStateCollect {
    fn collect_draw_state(&mut self) -> impl Iterator<Item=&mut DrawStateDiff>;
}