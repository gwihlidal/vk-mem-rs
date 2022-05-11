use crate::ffi;
use crate::Allocator;
use ash::prelude::VkResult;
use ash::vk;

pub use ffi::VmaDefragmentationMove as DefragmentationMove;
pub use ffi::VmaDefragmentationStats as DefragmentationStats;
pub struct DefragmentationContext<'a> {
    allocator: &'a Allocator,
    raw: ffi::VmaDefragmentationContext,
}

impl<'a> Drop for DefragmentationContext<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaEndDefragmentation(self.allocator.internal, self.raw, std::ptr::null_mut());
        }
    }
}

impl<'a> DefragmentationContext<'a> {
    /// Ends defragmentation process.
    pub fn end(self) -> DefragmentationStats {
        let mut stats = DefragmentationStats {
            bytesMoved: 0,
            bytesFreed: 0,
            allocationsMoved: 0,
            deviceMemoryBlocksFreed: 0,
        };
        unsafe {
            ffi::vmaEndDefragmentation(self.allocator.internal, self.raw, &mut stats);
        }
        std::mem::forget(self);
        stats
    }

    /// Returns `false` if no more moves are possible or `true` if more defragmentations are possible.
    pub fn begin_pass(&self, mover: impl FnOnce(&mut [DefragmentationMove]) -> ()) -> bool {
        let mut pass_info = ffi::VmaDefragmentationPassMoveInfo {
            moveCount: 0,
            pMoves: std::ptr::null_mut(),
        };
        let result = unsafe {
            ffi::vmaBeginDefragmentationPass(self.allocator.internal, self.raw, &mut pass_info)
        };
        if result == vk::Result::SUCCESS {
            return false;
        }
        debug_assert_eq!(result, vk::Result::INCOMPLETE);
        let moves = unsafe {
            std::slice::from_raw_parts_mut(pass_info.pMoves, pass_info.moveCount as usize)
        };
        mover(moves);

        let result = unsafe {
            ffi::vmaEndDefragmentationPass(self.allocator.internal, self.raw, &mut pass_info)
        };

        return result == vk::Result::INCOMPLETE;
    }
}

impl Allocator {
    /// Begins defragmentation process.
    ///
    /// ## Returns
    /// `VK_SUCCESS` if defragmentation can begin.
    /// `VK_ERROR_FEATURE_NOT_PRESENT` if defragmentation is not supported.
    pub unsafe fn begin_defragmentation(
        &self,
        info: &ffi::VmaDefragmentationInfo,
    ) -> VkResult<DefragmentationContext> {
        let mut context: ffi::VmaDefragmentationContext = std::ptr::null_mut();

        ffi::vmaBeginDefragmentation(self.internal, info, &mut context).result()?;

        Ok(DefragmentationContext {
            allocator: self,
            raw: context,
        })
    }
}
