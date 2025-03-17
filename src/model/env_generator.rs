use super::obj::{Indices, Obj};

use std::num::NonZeroU32;

use glam::Vec3;

pub fn default_env() -> Obj {
    let walls = [
        // big wall for images
        Wall { start: [6., -14.], end: [6.2, 0.], height: 3. },

        /* currently replaced by some pillar shaders
        // podests row left
        Wall { start: [-3., -1.], end: [-2.,  0.], height: 1. },
        Wall { start: [-3., -6.], end: [-2., -5.], height: 1. },
        // podests row right
        Wall { start: [ 2., -1.], end: [ 3.,  0.], height: 1. },
        Wall { start: [ 2., -6.], end: [ 3., -5.], height: 1. },
        */
    ];
    generate_env(
        [-10.0, 0.0, -15.0],
        [  8.2, 0.0,   4.2],
        &walls,
    )
}

fn add_surface(
    start: Vec3,
    end: Vec3,
    dir_x: Vec3,
    dir_y: Vec3,
    vertices: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    faces: &mut Vec<([Indices; 3], Option<Indices>)>,
) {
    let vidx = vertices.len() as u32;
    let diag = end - start;
    let dimsf = [(diag * dir_x).element_sum().abs(), (diag * dir_y).element_sum().abs()];
    let dims = [dimsf[0] as u32, dimsf[1] as u32];
    let diff = [dimsf[0] - dims[0] as f32, dimsf[1] - dims[1] as f32];

    for y in 0..dims[1] + 1 {
        let mut pos = start + dir_y * y as f32;
        for _ in 0..dims[0] + 1 {
            vertices.push(pos.into());
            pos += dir_x;
        }
        if diff[0] > 0. {
            vertices.push((pos + dir_x * (diff[0] - 1.)).into());
        }
    }
    if diff[1] > 0. {
        let mut pos = start + dir_y * (dims[1] as f32 + diff[1]);
        for _ in 0..dims[0] + 1 {
            vertices.push(pos.into());
            pos += dir_x;
        }
        if diff[0] > 0. {
            vertices.push((pos + dir_x * (diff[0] - 1.)).into());
        }
    }

    let w = dims[0] + 1 + (diff[0] > 0.) as u32;
    let normal = {
        let vertices: [Vec3; 3] = [vidx, vidx + w, vidx + 1].map(|vidx| {
            vertices[vidx as usize].into()
        });
        let a = vertices[1] - vertices[0];
        let b = vertices[2] - vertices[1];
        let normal = a.cross(b).normalize().to_array();
        normals.push(normal);
        NonZeroU32::new(normals.len() as u32).unwrap()
    };
    for y in 0..dims[1] + (diff[1] > 0.) as u32 {
        for x in 0..w - 1 {
            let vidx = vidx + x + y * w;
            faces.push(indices_to_face([vidx, vidx + w, vidx + 1 + w, vidx + 1], normal));
        }
    }
}

fn generate_env(
    floor_start: [f32; 3],
    floor_end: [f32; 3],
    walls: &[Wall],
) -> Obj {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut normals = Vec::new();
    let tex_coords = Vec::new();

    // the floor
    add_surface(
        floor_start.into(),
        floor_end.into(),
        [1., 0., 0.].into(),
        [0., 0., 1.].into(),
        &mut vertices,
        &mut normals,
        &mut faces,
    );

    // the walls
    for wall in walls {
        // -z side
        add_surface(
            [wall.start[0],          0., wall.start[1]].into(),
            [  wall.end[0], wall.height, wall.start[1]].into(),
            [1., 0., 0.].into(),
            [0., 1., 0.].into(),
            &mut vertices,
            &mut normals,
            &mut faces,
        );
        // +x side
        add_surface(
            [  wall.end[0],          0., wall.start[1]].into(),
            [  wall.end[0], wall.height,   wall.end[1]].into(),
            [0., 0., 1.].into(),
            [0., 1., 0.].into(),
            &mut vertices,
            &mut normals,
            &mut faces,
        );
        // +z side
        add_surface(
            [  wall.end[0],          0.,   wall.end[1]].into(),
            [wall.start[0], wall.height,   wall.end[1]].into(),
            [-1., 0., 0.].into(),
            [ 0., 1., 0.].into(),
            &mut vertices,
            &mut normals,
            &mut faces,
        );
        // -x side
        add_surface(
            [wall.start[0],         0.,   wall.end[1]].into(),
            [wall.start[0], wall.height, wall.start[1]].into(),
            [0., 0., -1.].into(),
            [0., 1.,  0.].into(),
            &mut vertices,
            &mut normals,
            &mut faces,
        );
        // +y side
        add_surface(
            [  wall.start[0], wall.height, wall.start[1]].into(),
            [    wall.end[0], wall.height,   wall.end[1]].into(),
            [1., 0., 0.].into(),
            [0., 0., 1.].into(),
            &mut vertices,
            &mut normals,
            &mut faces,
        );
    }

    Obj { vertices, tex_coords, normals, faces }
}


fn indices_to_face(indices: [u32; 4], normal: NonZeroU32) -> ([Indices; 3], Option<Indices>) {
    let normal = Some(normal);
    let [a, b, c, d] = indices.map(|i| NonZeroU32::new(i + 1).unwrap());
    (
        [
            Indices { vertex: a, texture: None, normal },
            Indices { vertex: b, texture: None, normal },
            Indices { vertex: c, texture: None, normal },
        ],
        Some(Indices { vertex: d, texture: None, normal }),
    )
}

struct Wall {
    start: [f32; 2],
    end: [f32; 2],
    height: f32,
}
