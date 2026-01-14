use flume::bounded;
use pollster::FutureExt; // to block on the future

use std::error::Error;
use wgpu::{
    self,
    util::{BufferInitDescriptor, DeviceExt},
};

/// Strongly inspired from https://github.com/sotrh/learn-wgpu/blob/master/code/compute/src/introduction.rs
/// with the goal of learning the basics of gpu compute with wgpu
pub async fn merge_sort_gpu(input: Vec<i32>) -> Result<Vec<i32>, Box<dyn Error>> {
    //Strongly
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = instance.request_adapter(&Default::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

    let shader = device.create_shader_module(wgpu::include_wgsl!("merge_sort_shader.wgsl"));

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("merge sort compute pipeline"),
        layout: None,
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        cache: Default::default(),
    });

    let input_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(&input),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("temp"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });


    let mut encoder = device.create_command_encoder(&Default::default());

    {

        let mut bin_size = 1;



        while bin_size < input.len() {
            // Calculate the number of passes for 1 merge sort step on the full data
            let num_items_per_workgroup = 64 * bin_size * 2; // 64 threads, 2 bins per thread
            let num_dispatches = (input.len() / num_items_per_workgroup) as u32
                           + (input.len() % num_items_per_workgroup > 0) as u32;
            println!("num dispatches: {num_dispatches}");

            // Reinjecting the partially sorted data to the input buffer
            if bin_size > 1 {
                encoder.copy_buffer_to_buffer(&output_buffer, 0, &input_buffer, 0, output_buffer.size());
            }

            // Shared bin size buffer between all GPU threads
            let bin_size_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Size of a bin at this step"),
                contents: &[bin_size.try_into().unwrap()],
                usage: wgpu::BufferUsages::STORAGE,
            });

            // Initialize the pipeline at ech merge sort step
            // Necessary because we change the bin size buffer each time
            // Seems inefficient, is there a better way ?
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: bin_size_buffer.as_entire_binding(),
                    },
                ],
            });
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_dispatches, 1, 1);

            bin_size *= 2;
        }
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &temp_buffer, 0, output_buffer.size());

    queue.submit([encoder.finish()]);

    let (tx, rx) = bounded(1);

    temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        tx.send(result).unwrap()
    });

    device.poll(wgpu::PollType::wait_indefinitely())?;

    rx.recv_async().await??;

    let output_data = temp_buffer.get_mapped_range(..);

    Ok(Vec::from(bytemuck::cast_slice(&output_data)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_small_vec() {
        let test_vec = vec![15, 53, 1, 24, 3];
        assert_eq!(
            merge_sort_gpu(test_vec).block_on().unwrap(),
            vec![1, 3, 15, 24, 53]
        )
    }
}
