#!/usr/bin/env python3
"""Run the real GPU arena/upload tests without compiling the whole game.

Requires a host Vulkan/DX12 adapter with encoder timestamps. Generated files stay in .local/android.
No game, launcher, prepared resources or player profile is opened.
"""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    work = ROOT / '.local/android/gpu-tests'
    (work / 'src').mkdir(parents=True, exist_ok=True)
    (work / 'Cargo.toml').write_text('''[package]
name = "android-gpu-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
wgpu = { version = "30.0.1", default-features = false, features = ["std", "parking_lot", "vulkan", "dx12", "wgsl"] }
pollster = "0.4"
log = "0.4"
bytemuck = "1"
naga = { version = "30.0.1", features = ["wgsl-in"] }
glam = "0.30"
''', encoding='utf-8')
    modules = '\n'.join(
        f'#[path = "{(ROOT / "crates/horizon-client-render/src/gfx" / (name + ".rs")).as_posix()}"] pub mod {name};'
        for name in ('arena', 'upload', 'timing', 'frame_timing', 'opaque_output', 'terrain_source', 'entity_source')
    )
    (work / 'src/lib.rs').write_text('''#![allow(dead_code)]
#[path = "C:/Users/vasia/Documents/Projects/horizon-main/crates/horizon-client-render/src/overlay.rs"] pub mod overlay;
mod spans {
    pub fn time<T>(_: &str, f: impl FnOnce() -> T) -> T { f() }
}
mod gfx {
''' + modules + '''
#[test]
fn stage_timestamps_are_collected_without_extra_render_passes() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let features = timing::GpuTimer::features(&adapter);
    assert!(features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
        "this GPU timing regression needs encoder timestamps");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: features, ..Default::default()
    })).unwrap();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: 128 << 10, usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut timing = frame_timing::FrameTiming::new(&device, &queue).unwrap();
    for _ in 0..4 {
        let mut encoder = device.create_command_encoder(&Default::default());
        let target = (&view, wgpu::TextureFormat::Rgba8Unorm);
        timing.begin_prepare(&mut encoder);
        encoder.clear_buffer(&buffer, 0, None);
        timing.begin_world(&device, &mut encoder, target);
        encoder.clear_buffer(&buffer, 0, None);
        timing.screen(&device, &mut encoder, target);
        encoder.clear_buffer(&buffer, 0, None);
        timing.end(&device, &mut encoder, target);
        queue.submit([encoder.finish()]);
        timing.submitted(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        timing.submitted(&queue);
    }
    let summaries = timing.summaries();
    assert_eq!(summaries.iter().map(|s| s.0).collect::<Vec<_>>(),
        ["prepare_uploads", "world", "post_hand_hud"]);
    assert!(summaries.iter().all(|s| s.1.is_finite() && s.1 >= 0.0 && s.3 > 0));
}
}
#[test]
fn storage_pages_fit_portable_binding_limits() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("this regression test requires a host GPU adapter");
    eprintln!("GPU arena limit test: {}", adapter.get_info().name);
    // Reproduce Android's large allocation limit but smaller per-binding limit.
    let mut limits = wgpu::Limits::default();
    limits.max_buffer_size = adapter.limits().max_buffer_size.min(1 << 30);
    limits.max_storage_buffer_binding_size = 128 << 20;
    let (device, _) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: limits, ..Default::default()
    })).expect("portable GPU device");
    for stride in [4, 24] {
        let arena = gfx::arena::Arena::new(&device, stride, 256, true);
        assert!(arena.page_vertices() * stride <= device.limits().max_storage_buffer_binding_size);
        // Verify ordinary vertex pages retain the larger allocation limit.
        let vertex = gfx::arena::Arena::new(&device, stride, 256, false);
        assert!(vertex.page_vertices() >= arena.page_vertices());
    }
    let indices = gfx::arena::Arena::indices(&device, 256, true);
    assert!(indices.page_vertices() * 4 <= 128 << 20);
}
''', encoding='utf-8')
    with (work / 'src/lib.rs').open('a', encoding='utf-8') as output:
        shader = (ROOT / 'crates/horizon-client-render/src/terrain.wgsl').as_posix()
        output.write(f'\n#[test] fn terrain_shader_validates() {{\n'
                     f'let module = naga::front::wgsl::parse_str(include_str!("{shader}")).unwrap();\n'
                     'naga::valid::Validator::new(naga::valid::ValidationFlags::all(), '
                     'naga::valid::Capabilities::all()).validate(&module).unwrap();\n}\n')
    # Use the shader's real functions: duplicating their math in a GPU test
    # would miss changes made only to the shipped WGSL.
    terrain = (ROOT / 'crates/horizon-client-render/src/terrain.wgsl').read_text(encoding='utf-8')
    functions = terrain[terrain.index('fn sunrise_weight_full'):terrain.index("// The sky's colour")]
    (work / 'src/sunrise.wgsl').write_text('''const PI: f32 = 3.141592653589793;
struct Globals { sunrise: vec4<f32>, sunrise_fan: array<vec4<f32>,32> }
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage,read> rays: array<vec4<f32>>;
@group(0) @binding(2) var<storage,read_write> results: array<vec2<f32>>;
''' + functions + '''
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&rays) {
        let d = rays[id.x].xyz;
        results[id.x] = vec2(sunrise_weight_full(d), sunrise_weight(d));
    }
}
''', encoding='utf-8')
    with (work / 'src/lib.rs').open('a', encoding='utf-8') as output:
        for name, source in [
            ('sunrise_fan', ROOT / 'crates/horizon-client-render/src/gfx/sunrise_fan.rs'),
            ('sunrise_gpu', ROOT / 'tools/android_sunrise_gpu_test.rs'),
        ]:
            output.write(f'\n#[path = "{source.as_posix()}"] mod {name};\n')
    subprocess.run(['cargo', 'test', '--manifest-path', str(work / 'Cargo.toml'),
                    '--', '--nocapture', '--test-threads=1'], cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
