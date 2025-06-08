use anyhow::Result;



#[derive(Default)]
pub struct ImageInfo {
    pub flags: ash::vk::ImageCreateFlags,
    pub image_type: ash::vk::ImageType,
    pub format: ash::vk::Format,

}

#[derive(Default)]
pub struct ImageViewInfo {
    
}

pub struct ImageView {
    
}


impl crate::context::Context {
    pub fn create_image(&self, info: ImageInfo) -> Result<ImageView> {

        todo!()
    }

    pub fn create_image_view(&self, source: &ImageView, info: ImageViewInfo) -> Result<ImageView> {
        todo!()
    }
}

pub(crate) struct Image {
    info: ImageInfo,
    image: ash::vk::Image,
    allocation: vk_mem::Allocation,
}