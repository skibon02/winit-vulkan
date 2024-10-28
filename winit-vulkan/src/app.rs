pub trait AppTrait {
    fn new() -> Self;
    fn get_msaa_samples(&self) -> Option<usize>;
    fn get_vertex_data(&self) -> Vec<f32>;
    fn new_frame(&mut self) -> [f32; 3];
}