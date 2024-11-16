pub mod single_object;

pub enum DrawStateDiff {
    Create,
    Modify([f32; 3])
}

pub trait DrawStateCollect {
    fn collect_draw_state(&mut self) -> impl Iterator<Item=&mut DrawStateDiff>;
}