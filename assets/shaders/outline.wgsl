#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    prepass_utils,
    mesh_view_bindings,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, Vertex, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, Vertex, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct MyExtension {
    scale: f32,
    thresholds: vec4<f32>,
    // depth_threshold: f32,
    // depth_normal_threshold: f32,
    // depth_normal_threshold_scale: f32,
    // normal_threshold: f32,
    color: vec4<f32>,
    clip_to_view: mat4x4<f32>,
}

// @group(1) @binding(100) var<uniform> extension: MyExtension;

@group(2) @binding(100) var<uniform> scale: f32;
@group(2) @binding(101) var<uniform> depth_threshold: f32;
@group(2) @binding(102) var<uniform> depth_normal_threshold: f32;
@group(2) @binding(103) var<uniform> depth_normal_threshold_scale: f32;
@group(2) @binding(104) var<uniform> normal_threshold: f32;
@group(2) @binding(105) var<uniform> color: vec4<f32>;
@group(2) @binding(106) var<uniform> clip_to_view: mat4x4<f32>;

fn alphaBlend(top: vec4<f32>, bottom: vec4<f32>) -> vec4<f32> {
    var color: vec3<f32> = (top.rgb * top.a) + (bottom.rgb * (1 - top.a));
    var alpha: f32 = top.a + bottom.a * (1 - top.a);
    return vec4<f32>(color.xyz, alpha);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var edge_color: vec4<f32>;
    var half_scale_floor: f32 = floor(scale * 0.5);
    var half_scale_ceil: f32 = ceil(scale * 0.5);

	// no clue if using the position like this is at all correct
    let bottom_left_uv = in.position.xy - vec2<f32>(1.0, 1.0) * half_scale_floor;
    let top_right_uv = in.position.xy + vec2<f32>(1.0, 1.0) * half_scale_floor;
    let bottom_right_uv = in.position.xy + vec2<f32>(1.0 * half_scale_ceil, -1.0 * half_scale_floor);
    let top_left_uv = in.position.xy + vec2<f32>(-1.0 * half_scale_floor, 1.0 * half_scale_ceil);

    let depth0 = textureLoad(mesh_view_bindings::depth_prepass_texture, vec2<i32>(bottom_left_uv), 0);
    let depth1 = textureLoad(mesh_view_bindings::depth_prepass_texture, vec2<i32>(top_right_uv), 0);
    let depth2 = textureLoad(mesh_view_bindings::depth_prepass_texture, vec2<i32>(bottom_right_uv), 0);
    let depth3 = textureLoad(mesh_view_bindings::depth_prepass_texture, vec2<i32>(top_left_uv), 0);

    let normal0 = textureLoad(mesh_view_bindings::normal_prepass_texture, vec2<i32>(bottom_left_uv), 0);
    let normal1 = textureLoad(mesh_view_bindings::normal_prepass_texture, vec2<i32>(top_right_uv), 0);
    let normal2 = textureLoad(mesh_view_bindings::normal_prepass_texture, vec2<i32>(bottom_right_uv), 0);
    let normal3 = textureLoad(mesh_view_bindings::normal_prepass_texture, vec2<i32>(top_left_uv), 0);

    let view_normal = normal0 * 2 - 1;
    let view_dir = clip_to_view * in.position;
    let n_dot_v = 1 - dot(view_normal, -view_dir);

    let normal_threshold01 = saturate((n_dot_v - depth_normal_threshold) / (1 - depth_normal_threshold));
    let norm_threshold = normal_threshold01 * depth_normal_threshold_scale + 1;
    let dep_threshold = depth_threshold * depth0 * norm_threshold;

    let depthFiniteDifference0 = depth1 - depth0;
    let depthFiniteDifference1 = depth3 - depth2;

    var edge_depth = sqrt(pow(depthFiniteDifference0, 2.0) + pow(depthFiniteDifference1, 2.0)) * 100;
	// this functions slightly better with depth_threshold for some reason
    if (edge_depth > dep_threshold) {
        edge_depth = 1.0;
    } else {
        edge_depth = 0.0;
    }

    let normalFiniteDifference0 = normal1 - normal0;
    let normalFiniteDifference1 = normal3 - normal2;

    var edge_normal = sqrt(dot(normalFiniteDifference0, normalFiniteDifference0) + dot(normalFiniteDifference1, normalFiniteDifference1));

    if (edge_normal > normal_threshold) {
        edge_normal = 1.0;
    } else {
        edge_normal = 0.0;
    }
    var edge = max(edge_depth, edge_normal);
    edge_color = vec4(color.rgb, color.a * edge);

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);


    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    out.color = alphaBlend(edge_color, out.color);
#endif

    return out;
}

