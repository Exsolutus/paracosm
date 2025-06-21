use crate::queue::commands::CommandRecorder;

use anyhow::Result;


#[allow(private_bounds)]
pub trait ComputeCommands: CommandRecorder {
    fn dispatch(&mut self, x: u32, y: u32, z: u32) -> Result<()> {
        unsafe { self.device().cmd_dispatch(self.command_buffer()?, x, y, z) };

        Ok(())
    }
}
