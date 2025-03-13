use crate::{
    art::{ArtData, ArtObject, ArtOption},
    fs,
    model::obj::NormalizedObj,
    vulkan::HotShader,
};

use std::sync::Arc;

use egui::Color32;
use glam::{Mat4, Quat, Vec3};

pub fn get_art_objects() -> anyhow::Result<Vec<ArtObject>> {
    let model_square = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/square.obj")?)?);
    let model_cube = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/cube_inside.obj")?)?);
    let shader_2d = Arc::new(HotShader::new_vert("assets/shaders/art2d.vert"));
    let shader_3d = Arc::new(HotShader::new_vert("assets/shaders/art3d.vert"));
    let mut art_objects = vec![
        ArtObject {
            name: "Mandelbrot".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbrot.frag")),
            texture: None,
            options: vec![],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -1.5].into(),
            )),
            fn_update_data: None,
        },
        ArtObject {
            name: "Sdf Cat".to_owned(),
            model: model_square.clone(),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/sdf_cat.frag")),
            texture: None,
            options: vec![
                ArtOption::stroke("Color", 1., Color32::from_rgb(255, 76, 76)),
                ArtOption::slider_f32("Speed", 1., 0., 10.),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -4.5].into(),
            )),
            fn_update_data: None,
        },
        ArtObject {
            name: "Skybox".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/skybox.frag")),
            texture: None,
            options: vec![],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(100.),
                Quat::from_rotation_y(0_f32.to_radians()),
                [0., 0., 0.].into(),
            )),
            fn_update_data: Some(Box::new(|data, update| {
                data.matrix = Mat4::from_scale_rotation_translation(
                    Vec3::splat(100.),
                    Quat::from_rotation_y(update.skybox_rotation_angle),
                    [0., 0., 0.].into(),
                );
            })),
        },
        ArtObject {
            name: "Mandelbox".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbox.frag")),
            texture: None,
            options: vec![
                ArtOption::slider_f32("Scale", 3., -5., 5.),
                ArtOption::slider_i32("Iterations", 10, 1, 100),
                ArtOption::slider_f32("Epsilon", 0.0002, 0.00001, 0.001),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [-2.5, 1.51, -0.5].into(),
            )),
            fn_update_data: None,
        },
        ArtObject {
            name: "Menger Sponge".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mengersponge.frag")),
            texture: None,
            options: vec![
                ArtOption::slider_i32("Depth", 4, 1, 10),
                ArtOption::checkbox("Shadows", true),
                ArtOption::checkbox("MSAA", true),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [2.5, 1.51, -0.5].into(),
            )),
            fn_update_data: None,
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
                [-2.5, 1.51, -5.5].into(),
            )),
            fn_update_data: None,
        },
        ArtObject {
            name: "Gem".to_owned(),
            model: model_cube.clone(),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/gem.frag")),
            texture: None,
            options: vec![
                ArtOption::slider_i32("GemType", 1, 0, 1),
                ArtOption::slider_i32("ColorIndex", 2, 0, 7),
                ArtOption::slider_f32("Speed", 1., 0., 2.),
            ],
            data: ArtData::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [2.5, 1.51, -5.5].into(),
            )),
            fn_update_data: None,
        },
    ];

    for art in art_objects.iter_mut() {
        if art.options.is_empty() {
            continue;
        }

        let mut values = [0.; 4];
        let mut i = 0;
        for option in art.options.iter() {
            option.ty.save_value(&mut values, &mut i);
        }
        art.data.option_values = Some(values.into());
    }

    Ok(art_objects)
}
