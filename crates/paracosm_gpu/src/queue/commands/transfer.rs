use crate::{
    resource::BufferLabel, 
    queue::commands::CommandRecorder
};

use anyhow::Result;


#[allow(private_bounds)]
pub trait TransferCommands: CommandRecorder {
    fn upload_buffer<L: BufferLabel + 'static>(&mut self, label: L) -> Result<()> {
        let resource = self.resource(label)?;

        todo!();

        Ok(())
    }

    fn download_buffer<L: BufferLabel + 'static> (&mut self, label: L) -> Result<()> {
        todo!();
    }
}