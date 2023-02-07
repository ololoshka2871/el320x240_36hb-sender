// rust code representation of the ComputeConfig struct in the compute shader
#[repr(C)]
// Не дать rust переставить поля местами, будет как в C и как в шейдере
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ComputeConfig {
    pub width: u32,
    pub black_lvl: f32,
    pub white_lvl: f32,
}
