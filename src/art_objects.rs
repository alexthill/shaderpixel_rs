use crate::{
    art::{ArtData, ArtObject, ArtOption},
    fs,
    model::obj::NormalizedObj,
    vulkan::HotShader,
};

use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

use egui::Color32;
use glam::{Mat4, Quat, Vec3};

pub fn get_art_objects() -> anyhow::Result<Vec<ArtObject>> {
    let model_square = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/square.obj")?)?);
    let model_cube = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/cube_inside.obj")?)?);
    let model_teapot = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/teapot.obj")?)?);

    let shader_2d = Arc::new(HotShader::new_vert("assets/shaders/art2d.vert"));
    let shader_3d = Arc::new(HotShader::new_vert("assets/shaders/art3d.vert"));
    let shader_pillar = Arc::new(HotShader::new_frag("assets/shaders/pillar.frag"));

    let mut art_objects = vec![
        ArtObject {
            name: "Mandelbrot".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbrot.frag")),
            options: vec![],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -1.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Sdf Cat".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/sdf_cat.frag")),
            options: vec![
                ArtOption::stroke("Color", 1., Color32::from_rgb(255, 76, 76)),
                ArtOption::slider_f32("Speed", 1., 0., 10.),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -4.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Colorful Mozaic".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mozaic.frag")),
            options: vec![
                ArtOption::slider_f32("Speed", 1., 0., 10.),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -7.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Mirror".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mirror.frag")),
            options: vec![
                ArtOption::checkbox("Invert", false),
                ArtOption::checkbox("Depth", false),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::new(6.0, 1., 1.0),
                Quat::from_rotation_y(-90_f32.to_radians()),
                [-5.99, 1.0, -6.0].into(),
            )),
            is_mirror: true,
            ..Default::default()
        },
        ArtObject {
            name: "Portal".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/portal.frag")),
            options: vec![
                ArtOption::slider_i32("Ball number", 5, 1, 100),
                ArtOption::slider_i32("Rail Rotation", 3, -10, 10),
                ArtOption::slider_f32("Ball Size", 0.05, 0., 0.2),
                ArtOption::slider_f32("Rail Size", 0.06, 0., 0.1),
                ArtOption::slider_f32("Rail width", 0.011, 0., 0.2),
                ArtOption::slider_i32("ColorIndex", 1, 0, 7),
                ArtOption::checkbox("Invert", false),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(1.0),
                Quat::from_rotation_y(90_f32.to_radians()),
                [6.0, 1.501, 2.0].into(),
            )),
            fn_update_data: Some(Box::new(|data, update| {
                if goes_through_rect(update.old_position, update.new_position, data.matrix) {
                    data.inside_portal = !data.inside_portal;
                }
            })),
            container_scale: Vec3::new(1., 1.5, 0.5),
            ..Default::default()
        },
        ArtObject {
            name: "Portalbox".to_owned(),
            model: model_cube.clone(),
            fn_update_data: Some(Box::new(|data, _| {
                // draw after all other shaders
                data.dist_to_camera_sqr = -1.;
            })),
            enable_pipeline: false,
            enable_depth_test: false,
            container_scale: Vec3::splat(100.),
            ..Default::default()
        },
        ArtObject {
            name: "Player".to_owned(),
            model: model_teapot.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/player.frag")),
            fn_update_data: Some(Box::new(|data, update| {
                let matrix = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.4),
                    Quat::from_rotation_y(90_f32.to_radians()),
                    Vec3::new(0.0, -1.0, 1.0),
                );
                data.dist_to_camera_sqr = 0.;
                data.matrix = Mat4::IDENTITY
                    * Mat4::from_translation(update.camera.position)
                    * Mat4::from_rotation_y(-update.camera.angle_yaw)
                    * matrix;
            })),
            ..Default::default()
        },
        ArtObject {
            name: "Skybox".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/skybox.frag")),
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(100.),
                Quat::from_rotation_y(0_f32.to_radians()),
                [0., 0., 0.].into(),
            )),
            fn_update_data: Some(Box::new(|data, update| {
                // draw before all other shaders
                data.dist_to_camera_sqr = f32::MAX;
                data.matrix = Mat4::from_scale_rotation_translation(
                    Vec3::splat(100.),
                    Quat::from_rotation_y(update.skybox_rotation_angle),
                    [0., 0., 0.].into(),
                );
            })),
            ..Default::default()
        },
        ArtObject {
            name: "Mandelbox".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbox.frag")),
            options: vec![
                ArtOption::slider_f32("Scale", 3., -5., 5.),
                ArtOption::slider_i32("Iterations", 10, 1, 100),
                ArtOption::slider_f32_log("Epsilon", 0.0002, 0.000001, 0.001),
                ArtOption::checkbox("Shadows", false),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [-2.5, 1.5, -0.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Mandelbulb".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbulb.frag")),
            options: vec![
                ArtOption::slider_i32("Power", 8, 1, 20),
                ArtOption::slider_i32("Iterations", 10, 1, 100),
                ArtOption::slider_f32_log("Epsilon", 0.0002, 0.000001, 0.001),
                ArtOption::slider_i32("ColorIndex", 3, 0, 7),
                ArtOption::checkbox("Shadows", true),
                ArtOption::checkbox("Animate", true),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [-2.5, 1.5, -5.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Menger Sponge".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mengersponge.frag")),
            options: vec![
                ArtOption::slider_i32("Depth", 4, 1, 10),
                ArtOption::checkbox("Shadows", true),
                ArtOption::checkbox("MSAA", true),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [-2.5, 1.5, -10.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Solar System".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/solar.frag")),
            texture: Some("assets/downloads/earth.jpg".into()),
            options: vec![
                ArtOption::slider_f32("Speed", 1., 0., 10.),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [2.5, 1.5, -10.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Gem".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/gem.frag")),
            options: vec![
                ArtOption::slider_i32("GemType", 1, 0, 1),
                ArtOption::slider_i32("ColorIndex", 2, 0, 7),
                ArtOption::slider_f32("Speed", 1., 0., 2.),
                ArtOption::checkbox("Diffuse", true),
                ArtOption::checkbox("Specular", true),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [2.5, 1.5, -0.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Cloudy Cube".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/cloudycube.frag")),
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [2.5, 1.5, -5.5].into(),
            )),
            ..Default::default()
        },
    ];

    let pillars = [
        [-2.5, 0.5, -10.5],
        [ 2.5, 0.5, -10.5],
        [-2.5, 0.5,  -5.5],
        [ 2.5, 0.5,  -5.5],
        [-2.5, 0.5,  -0.5],
        [ 2.5, 0.5,  -0.5],
    ];
    art_objects.extend(pillars.into_iter().enumerate().map(|(i, pillar_pos)| {
        ArtObject {
            name: format!("Pillar {i:2}"),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: shader_pillar.clone(),
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::new(0.53, 0.499, 0.53),
                Quat::from_rotation_y(0_f32.to_radians()),
                pillar_pos.into(),
            )),
            ..Default::default()
        }
    }));

    for art in art_objects.iter_mut() {
        art.save_options();
    }

    Ok(art_objects)
}

fn goes_through_rect(p0: Vec3, p1: Vec3, matrix: Mat4) -> bool {
    let dir = p1 - p0;
    let p_norm = matrix.inverse().transpose().transform_vector3(Vec3::new(0., 0., 1.));
    let p_pos = matrix.transform_point3(Vec3::new(0., 0., 0.));
    let dot = p_norm.dot(dir);
    if dot == 0.0 {
        return false; // segment [p0,p1] parallel to plane
    }
    let w = p0 - p_pos;
    let fac = -p_norm.dot(w) / dot;
    if !(0.0..1.0).contains(&fac) {
        return false; // segment [p0,p1] not passing through plane
    }
    let inter = p0 + dir * fac;
    let corner0 = matrix.transform_point3(Vec3::new(-1., -1., 0.) * FRAC_1_SQRT_2);
    let corner1 = matrix.transform_point3(Vec3::new( 1.,  1., 0.) * FRAC_1_SQRT_2);
    (corner0 - inter).dot(corner1 - inter) < 0.0
}
