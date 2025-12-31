use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::{TransferMode, buffer::BufferInfo}
};

use hello_compute_shared::PushConstant;


const APPNAME: &str = "Paracosm GPU Hello Compute";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);

const NUMBERS: [u32; 4] = [1, 2, 3, 4];
const OVERFLOW: u32 = 0xffffffff;

#[derive(PipelineLabel)] struct HelloCompute;
#[derive(BufferLabel)] struct NumbersBuffer;

fn main() {
    {
        println!("Hello Compute!");

        let steps = execute_gpu(&NUMBERS);

        // Output results
        let disp_steps: Vec<String> = steps.iter()
            .map(|&n| match n {
                OVERFLOW => "OVERFLOW".to_string(),
                _ => n.to_string(),
            })
            .collect();
        println!("Steps: [{}]", disp_steps.join(", "));
    }
}

fn execute_gpu(
    numbers: &[u32]
) -> Vec<u32> {
    // Create GPU context
    let mut context = Context::new(
        ContextInfo {
            application_name: APPNAME.into(),
            application_version: APPVER,
            ..Default::default()
        },
        None
    ).unwrap();

    // Create numbers buffer
    let mut numbers_buffer = context.create_buffer(BufferInfo {
        transfer_mode: TransferMode::Stream,
        size: size_of_val(numbers),
        shader_mutable: true,
        #[cfg(debug_assertions)] debug_name: "NumbersBuffer"
    }).unwrap();
    let memory = context.get_buffer_memory_mut::<[u32; 4]>(&mut numbers_buffer).unwrap();
    for (index, element) in memory.iter_mut().enumerate() {
        *element = numbers[index];
    }

    context.set_buffer_label(NumbersBuffer, &numbers_buffer).unwrap();

    // Create shader pipelines
    context.create_pipeline(HelloCompute, PipelineInfo::Compute {
        shader_source: ShaderSource::Crate("examples/gpu/hello_compute/shaders".into()),
        entry_point: "main_cs",
    }).unwrap();


    // Add nodes to frame graph
    context.add_nodes(Queue::Compute,
        |mut interface: ComputeInterface, numbers_buffer: Write<NumbersBuffer>| {
            interface.bind_pipeline(HelloCompute).unwrap();
            interface.set_push_constant(PushConstant { 
                descriptor_index: numbers_buffer.buffer().descriptor_index
            }).unwrap();
            interface.dispatch(NUMBERS.len() as u32, 1, 1);
        }
    ).unwrap();
 
    context.add_submit(
        Queue::Compute,
        None
    ).unwrap();


    // Build and run frame graph
    context.execute().unwrap();

    // Wait for primary device to finish execution
    context.wait_idle();

    let result = context.get_buffer_memory::<[u32; 4]>(&numbers_buffer).unwrap().to_vec();

    context.destroy_buffer(numbers_buffer).unwrap();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_1() {
        let input = vec![1, 2, 3, 4];
        assert_eq!(execute_gpu(&input), vec![0, 1, 7, 2]);
    }

    #[test]
    fn test_compute_2() {
        let input = vec![5, 23, 10, 9];
        assert_eq!(execute_gpu(&input), vec![5, 15, 6, 19]);
    }

    #[test]
    fn test_compute_overflow() {
        let input = vec![77031, 837799, 8400511, 63728127];
        assert_eq!(execute_gpu(&input), vec![350, 524, OVERFLOW, OVERFLOW]);
    }
}
