use crate::{
    resource::buffer::BufferLabel, 
    queue::commands::CommandRecorder
};

use anyhow::Result;


#[allow(private_bounds)]
pub trait TransferCommands: CommandRecorder {
    fn upload_buffer<L: BufferLabel + 'static>(&mut self, label: L) -> Result<()> {
        todo!();

        Ok(())
    }

    fn download_buffer<L: BufferLabel + 'static> (&mut self, label: L) -> Result<()> {
        todo!();
    }
}