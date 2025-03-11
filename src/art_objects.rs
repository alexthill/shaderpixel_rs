use crate::{
    art::{ArtObject, ArtOption},
    fs,
    model::obj::NormalizedObj,
    vulkan::HotShader,
};

use std::sync::{Arc, RwLock};

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
            matrix: Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -1.5].into(),
            ),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbrot.frag")),
            options: vec![],
            option_values: None,
        },
        ArtObject {
            name: "Sdf Cat".to_owned(),
            model: model_square.clone(),
            matrix: Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(90_f32.to_radians()),
                [5.99, 1.5, -4.5].into(),
            ),
            shader_vert: shader_2d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/sdf_cat.frag")),
            options: vec![
                ArtOption::stroke("Color", 1., Color32::from_rgb(255, 76, 76)),
                ArtOption::slider("Speed", 1., 0., 10.),
            ],
            option_values: None,
        },
        ArtObject {
            name: "Mandelbox".to_owned(),
            model: model_cube.clone(),
            matrix: Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_rotation_y(0_f32.to_radians()),
                [-2.5, 1.51, -0.5].into(),
            ),
            shader_vert: shader_3d.clone(),
            shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbox.frag")),
            options: vec![
                ArtOption::slider("Scale", 3., -5., 5.),
                ArtOption::slider("Iterations", 10., 1., 100.),
                ArtOption::slider("Epsilon", 0.0002, 0.00001, 0.001),
            ],
            option_values: None,
        },
    ];

    for art in art_objects.iter_mut() {
        assert!(art.option_values.is_none());
        let mut values = [0.; 4];
        let mut i = 0;
        for option in art.options.iter() {
            option.ty.save_value(&mut values, &mut i);
        }
        art.option_values = Some(Arc::new(RwLock::new(values.into())));
    }

    Ok(art_objects)
}
