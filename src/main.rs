use std::f32::consts::PI;

use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use bevy::{
    core_pipeline::{
        prepass::{DepthPrepass, NormalPrepass},
        Skybox,
    },
    pbr::{ExtendedMaterial, MaterialExtension, OpaqueRendererMethod},
    prelude::*,
    render::render_resource::*,
    scene::SceneInstance,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Thresholds>()
        .register_type::<Thresholds>()
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, MyExtension>,
        >::default())
        .add_plugins(ResourceInspectorPlugin::<Thresholds>::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_things)
        .add_systems(Update, update_mats)
        .add_systems(
            Update,
            customize_scene_materials.run_if(any_with_component::<CustomizeMaterial>),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    ass: ResMut<AssetServer>,
) {
    let location = Transform::from_xyz(0., 0., 5.0).looking_at(Vec3::ZERO, Vec3::Y);
    let mat = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            // can be used in forward or deferred mode.
            opaque_render_method: OpaqueRendererMethod::Auto,
            // in deferred mode, only the PbrInput can be modified (uvs, color and other material properties),
            // in forward mode, the output can also be modified after lighting is applied.
            // see the fragment shader `extended_material.wgsl` for more info.
            // Note: to run in deferred mode, you must also add a `DeferredPrepass` component to the camera and either
            // change the above to `OpaqueRendererMethod::Deferred` or add the `DefaultOpaqueRendererMethod` resource.
            double_sided: true,
            ..Default::default()
        },
        extension: MyExtension {
            scale: 2.0,
            depth_threshold: 0.2,
            depth_normal_threshold: 0.5,
            depth_normal_threshold_scale: 7.0,
            normal_threshold: 0.4,
            // TODO: fix me
            color: Color::BLACK.rgba_to_vec4(),
            clip_to_view: location.compute_matrix().inverse(),
        },
    });
    commands.insert_resource(Thresholds {
        scale: 2,
        depth_threshold: 0.2,
        depth_normal_threshold: 0.5,
        depth_normal_threshold_scale: 7.0,
        normal_threshold: 0.4,
        color: Color::BLACK,
    });
    commands.insert_resource(Mat(mat.clone()));

    // stairs
    // NOTE: uvs are messed up on this one
    let mut stairs = Transform::from_xyz(-5.0, 0., -10.0);
    stairs.rotate_y(PI * 0.8);
    stairs.rotate_x(-PI / 11.);
    stairs.rotate_z(-PI / 8.);

    let torus = Transform::from_xyz(8.0, 2.0, -15.0);

    let mut teapot = Transform::from_xyz(0., -3., -10.);
    teapot.rotate_x(PI / 6.);
    teapot.rotate_z(-PI / 6.);

    commands.spawn((
        SceneBundle {
            scene: ass.load("models/stairs.glb#Scene0"),
            transform: stairs,
            ..default()
        },
        CustomizeMaterial,
    ));

    // torus
    commands.spawn((
        SceneBundle {
            scene: ass.load("models/torus.glb#Scene0"),
            transform: torus,
            ..default()
        },
        CustomizeMaterial,
    ));

    // teapot
    commands.spawn((
        SceneBundle {
            scene: ass.load("models/teapot.glb#Scene0"),
            transform: teapot,
            ..default()
        },
        CustomizeMaterial,
    ));

    // light
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        Rotate,
    ));

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: location,
            projection: Projection::Perspective(PerspectiveProjection {
                near: 1.0,
                ..default()
            }),
            ..default()
        },
        Skybox {
            image: ass.load("textures/Ryfjallet_cubemap_bc7.ktx2"),
            brightness: 250.0,
        },
        DepthPrepass,
        NormalPrepass,
        PanOrbitCamera::default(),
    ));
}

fn customize_scene_materials(
    unloaded_instances: Query<(Entity, &SceneInstance), With<CustomizeMaterial>>,
    handles: Query<(Entity, &Handle<StandardMaterial>)>,
    pbr_materials: Res<Assets<StandardMaterial>>,
    scene_manager: Res<SceneSpawner>,
    mut cmds: Commands,
    mat: Res<Mat>,
) {
    for (entity, instance) in unloaded_instances.iter() {
        if scene_manager.instance_is_ready(**instance) {
            cmds.entity(entity).remove::<CustomizeMaterial>();
        }
        // Iterate over all entities in scene (once it's loaded)
        let handles = handles.iter_many(scene_manager.iter_instance_entities(**instance));
        for (entity, material_handle) in handles {
            let Some(_material) = pbr_materials.get(material_handle) else {
                continue;
            };
            cmds.entity(entity)
                .insert(mat.0.clone())
                .remove::<Handle<StandardMaterial>>();
        }
    }
}

fn update_mats(
    handles: Query<&Handle<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    thresholds: Res<Thresholds>,
    camera: Query<&Transform, With<Camera>>,
) {
    for hand in &handles {
        let Some(material) = materials.get_mut(hand) else {
            continue;
        };
        material.extension.scale = thresholds.scale as f32;
        material.extension.depth_threshold = thresholds.depth_threshold;
        material.extension.normal_threshold = thresholds.normal_threshold;
        material.extension.depth_normal_threshold = thresholds.depth_normal_threshold;
        material.extension.depth_normal_threshold_scale = thresholds.depth_normal_threshold_scale;
        material.extension.color = thresholds.color.rgba_to_vec4();
        material.extension.clip_to_view = camera.single().compute_matrix().inverse();
    }
}

#[derive(Resource, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Thresholds {
    #[inspector(min = 1, max = 20)]
    scale: u8,
    #[inspector(min = 0.0, max = 20.0)]
    depth_threshold: f32,
    #[inspector(min = 0.0, max = 20.0)]
    depth_normal_threshold: f32,
    #[inspector(min = 0.0, max = 20.0)]
    depth_normal_threshold_scale: f32,
    #[inspector(min = 0.0, max = 20.0)]
    normal_threshold: f32,
    color: Color,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            scale: 2,
            depth_threshold: 0.2,
            depth_normal_threshold: 0.5,
            depth_normal_threshold_scale: 7.0,
            normal_threshold: 0.4,
            color: Color::BLACK,
        }
    }
}

#[derive(Resource)]
struct Mat(Handle<ExtendedMaterial<StandardMaterial, MyExtension>>);

#[derive(Component)]
struct CustomizeMaterial;

#[derive(Component)]
struct Rotate;

fn rotate_things(mut q: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    for mut t in &mut q {
        t.rotate_y(time.delta_seconds());
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct MyExtension {
    #[uniform(100)]
    scale: f32,
    #[uniform(101)]
    depth_threshold: f32,
    #[uniform(102)]
    depth_normal_threshold: f32,
    #[uniform(103)]
    depth_normal_threshold_scale: f32,
    #[uniform(104)]
    normal_threshold: f32,
    #[uniform(105)]
    color: Vec4, // vec4<f32>
    #[uniform(106)]
    clip_to_view: Mat4, // mat4x4
}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }
}
