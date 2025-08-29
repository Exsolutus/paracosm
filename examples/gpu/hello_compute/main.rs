use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::TransferMode
};

use hello_compute_shared::PushConstant;


const APPNAME: &str = "Paracosm GPU Hello Compute";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);

const NUMBERS: [u32; 4] = [1, 2, 3, 4];
const OVERFLOW: u32 = 0xffffffff;

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
    #[derive(BufferLabel)] struct NumbersBuffer;

    context.create_buffer(
        NumbersBuffer,
        TransferMode::Stream,
        size_of_val(numbers)
    ).unwrap();
    let memory = context.get_buffer_memory_mut::<[u32; 4]>(NumbersBuffer).unwrap();
    for (index, element) in memory.iter_mut().enumerate() {
        *element = numbers[index];
    }

    // Create shader pipelines
    let shader_module_a = context.load_shader_module(ShaderSource::Crate("examples/gpu/hello_compute/shaders".into())).unwrap();

    #[derive(PipelineLabel)] struct HelloCompute;
    context.create_pipeline(HelloCompute, PipelineInfo::Compute {
        shader_module: shader_module_a,
        entry_point: "main_cs",
    }).unwrap();


    // Add nodes to frame graph
    context.add_nodes(Queue::Compute,
        |mut interface: ComputeInterface, numbers_buffer: Write<NumbersBuffer>| {
            interface.bind_pipeline(HelloCompute).unwrap();
            interface.set_push_constant(PushConstant { descriptor_index: *numbers_buffer }).unwrap();
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

    let result = context.get_buffer_memory::<[u32; 4]>(NumbersBuffer).unwrap().to_vec();

    context.destroy_buffer(NumbersBuffer).unwrap();

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
