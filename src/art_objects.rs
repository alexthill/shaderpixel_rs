use crate::{
    art::{ArtData, ArtObject, ArtOption},
    fs,
    model::obj::NormalizedObj,
    vulkan::HotShader,
};

use std::sync::Arc;

use egui::Color32;
use glam::{Mat4, Quat, Vec3, Vec4};

pub fn get_art_objects() -> anyhow::Result<Vec<ArtObject>> {
    let model_square = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/square.obj")?)?);
    let model_cube = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/cube_inside.obj")?)?);

    let shader_2d = Arc::new(HotShader::new_vert("assets/shaders/art2d.vert"));
    let shader_3d = Arc::new(HotShader::new_vert("assets/shaders/art3d.vert"));
    let shader_portal = Arc::new(HotShader::new_frag("assets/shaders/portal.frag"));
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
            name: "Mirror".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mirror.frag")),
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -7.5].into(),
            )),
            ..Default::default()
        },
        ArtObject {
            name: "Portal".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: shader_portal.clone(),
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [6.0, 1.001, 2.0].into(),
            )),
            fn_update_data: Some(Box::new(|data, update| {
                if goes_through_rect(update.old_position, update.new_position, data.matrix) {
                    data.inside_portal = !data.inside_portal;
                }
            })),
            container_scale: Vec3::new(1., 2., 0.01),
            ..Default::default()
        },
        ArtObject {
            name: "Portalbox".to_owned(),
            model: model_cube.clone(),
            fn_update_data: Some(Box::new(|data, _| {
                // draw after all other shaders
                data.dist_to_camera_sqr = -1.;
                data.option_values[3] = 1.;
            })),
            enable_pipeline: false,
            enable_depth_test: false,
            container_scale: Vec3::splat(100.),
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
                ArtOption::checkbox("Shadows", true),
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
        if art.options.is_empty() {
            continue;
        }

        let mut values = [0.; 4];
        let mut i = 0;
        for option in art.options.iter() {
            option.ty.save_value(&mut values, &mut i);
        }
        art.data.option_values = values.into();
    }

    Ok(art_objects)
}

fn goes_through_rect(p0: Vec3, p1: Vec3, matrix: Mat4) -> bool {
    const EPS: f32 = 0.001;
    let dir = p1 - p0;
    let p_norm = matrix.inverse().transpose().transform_vector3(Vec3::new(0., 0., 1.));
    let p_pos = matrix.transform_point3(Vec3::new(0., 0., 0.));
    let dot = p_norm.dot(dir);
    if dot.abs() < EPS {
        return false; // segment [p0,p1] parallel to plane
    }
    let w = p0 - p_pos;
    let fac = -p_norm.dot(w) / dot;
    if !(0.0..1.0).contains(&fac) {
        return false; // segment [p0,p1] not passing through plane
    }
    let inter = p0 + dir * fac;
    let corner0 = matrix * Vec4::new(-1., -2., 0., 1.);
    let corner1 = matrix * Vec4::new(1., 2., 0., 1.);
    (0..3).all(|i| {
        if corner0[i] < corner1[i] {
            (corner0[i] - EPS .. corner1[i] + EPS).contains(&inter[i])
        } else {
            (corner1[i] - EPS .. corner0[i] + EPS).contains(&inter[i])
        }
    })
}
