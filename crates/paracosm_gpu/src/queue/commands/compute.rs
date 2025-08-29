use crate::queue::commands::CommandRecorder;


#[allow(private_bounds)]
pub trait ComputeCommands: CommandRecorder {
    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        unsafe { self.device().cmd_dispatch(self.command_buffer(), x, y, z) };
    }
}
