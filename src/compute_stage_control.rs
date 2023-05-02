use winit::dpi::PhysicalSize;

use crate::{args::DitherAlgorithm, texture::Texture};

mod ordered;
mod threshold;
mod pinwheel;

pub(crate) trait ComputeStageControl {
    fn configure(
        &self,
        device: &wgpu::Device,
        camera_texture: &Texture,
        display_textures: &[Texture],
        lvls: (f32, f32),
    ) -> (
        wgpu::ComputePipeline,
        wgpu::Buffer,
        wgpu::BindGroup,
        Vec<wgpu::BindGroup>,
        wgpu::Buffer,
    );
    fn call_dispatch(&self, cs: &mut wgpu::ComputePass);
}

pub(crate) fn create_compute_stage(
    algo: DitherAlgorithm,
    output_size: PhysicalSize<u32>,
) -> Box<dyn ComputeStageControl> {
    match algo {
        DitherAlgorithm::Threshold => {
            Box::new(threshold::ThresholdComputeStageControl::new(output_size))
        }
        DitherAlgorithm::Ordered => Box::new(ordered::OrderedComputeStageControl::new(output_size)),
        DitherAlgorithm::Pinwheel => Box::new(pinwheel::PinwheelComputeStageControl::new(output_size)),
    }
}
