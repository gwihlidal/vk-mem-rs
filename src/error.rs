use ash::vk;
use std::fmt::Debug;
use std::panic::Location;

pub type VmaResult<T> = Result<T, VmaError>;

#[derive(thiserror::Error)]
pub enum VmaError {
    #[error("vk-mem: Allocator isn't initialized or has been destroyed.")]
    NotInitialized(),

    #[error("vk-mem: Unable to dynamically resolve vulkan functions: {0}")]
    UnsupportedVulkanFeature(&'static str),

    #[error("vk-mem: Invalid parameter error: {0}")]
    InvalidParameter(&'static str),

    #[error("vk-mem: {0} at {0}")]
    PreviouslyUnmapped(&'static str, &'static Location<'static>),

    #[error("vk-mem: {0} at {0}")]
    UnableToMapMemory(&'static str, &'static Location<'static>),

    #[error("vk-mem: {0} at {0}")]
    MappingNotHostVisible(&'static str, &'static Location<'static>),

    #[error("vk-mem: FFI function returned an unexpected result.")]
    UnknownError(),

    #[error("vk-mem: Not a valid buffer.")]
    InvalidBuffer(),

    #[error("vk-mem: Not a valid image.")]
    InvalidImage(),

    #[error("vk-mem: Vulkan error {0}")]
    VulkanError(vk::Result),
}

impl Debug for VmaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        Ok(())
    }
}

impl From<vk::Result> for VmaError {
    fn from(result: vk::Result) -> Self {
        VmaError::VulkanError(result)
    }
}
