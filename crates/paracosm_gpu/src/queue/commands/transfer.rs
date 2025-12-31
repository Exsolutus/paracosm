use crate::{
    queue::commands::CommandRecorder, resource::SyncLabel
};

use anyhow::Result;


#[allow(private_bounds)]
pub trait TransferCommands: CommandRecorder {
    fn upload_buffer<L: SyncLabel + 'static>(&mut self, label: L) -> Result<()> {
        todo!();

        Ok(())
    }

    fn download_buffer<L: SyncLabel + 'static> (&mut self, label: L) -> Result<()> {
        todo!();
    }
}