use super::obj::{Indices, Obj};

use std::num::NonZeroU32;

use glam::Vec3;

pub fn default_env() -> Obj {
    let podests = [
        [-3., -1.], [2., -1.],
        [-3., -6.], [2., -6.],
    ];
    let walls = [
        Wall { start: [6., -9.], end: [6.2, 0.], height: 3. },
    ];
    generate_env(
        [-10.0, 0.0, -10.0],
        [  8.2, 0.0,   4.2],
        &podests,
        &walls,
    )
}

fn add_surface(
    start: Vec3,
    end: Vec3,
    dir_x: Vec3,
    dir_y: Vec3,
    vertices: &mut Vec<[f32; 3]>,
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
    for y in 0..dims[1] + (diff[1] > 0.) as u32 {
        for x in 0..w - 1 {
            let vidx = vidx + x + y * w;
            faces.push(indices_to_face([vidx, vidx + w, vidx + 1 + w, vidx + 1]));
        }
    }
}

fn generate_env(
    floor_start: [f32; 3],
    floor_end: [f32; 3],
    podests: &[[f32; 2]],
    walls: &[Wall],
) -> Obj {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let tex_coords = Vec::new();

    // the floor
    add_surface(
        floor_start.into(),
        floor_end.into(),
        [1., 0., 0.].into(),
        [0., 0., 1.].into(),
        &mut vertices,
        &mut faces,
    );

    // the podests
    for podest in podests {
        let vidx = vertices.len() as u32;
        for z in 0..2 {
            for x in 0..2 {
                vertices.push([podest[0] + x as f32, 0., podest[1] + z as f32]);
                vertices.push([podest[0] + x as f32, 1., podest[1] + z as f32]);
            }
        }
        faces.push(indices_to_face([vidx + 1, vidx + 5, vidx + 7, vidx + 3]));
        faces.push(indices_to_face([vidx    , vidx + 1, vidx + 3, vidx + 2]));
        faces.push(indices_to_face([vidx + 2, vidx + 3, vidx + 7, vidx + 6]));
        faces.push(indices_to_face([vidx + 6, vidx + 7, vidx + 5, vidx + 4]));
        faces.push(indices_to_face([vidx + 4, vidx + 5, vidx + 1, vidx    ]));
    }

    // the walls
    for wall in walls {
        add_surface(
            [wall.start[0],         0.0, wall.start[1]].into(),
            [  wall.end[0], wall.height, wall.start[1]].into(),
            [1., 0., 0.].into(),
            [0., 1., 0.].into(),
            &mut vertices,
            &mut faces,
        );
        add_surface(
            [  wall.end[0],         0.0, wall.start[1]].into(),
            [  wall.end[0], wall.height,   wall.end[1]].into(),
            [0., 0., 1.].into(),
            [0., 1., 0.].into(),
            &mut vertices,
            &mut faces,
        );
        add_surface(
            [  wall.end[0],         0.0,   wall.end[1]].into(),
            [wall.start[0], wall.height,   wall.end[1]].into(),
            [-1., 0., 0.].into(),
            [ 0., 1., 0.].into(),
            &mut vertices,
            &mut faces,
        );
        add_surface(
            [wall.start[0],         0.0,   wall.end[1]].into(),
            [wall.start[0], wall.height, wall.start[1]].into(),
            [0., 0., -1.].into(),
            [0., 1.,  0.].into(),
            &mut vertices,
            &mut faces,
        );
    }

    Obj { vertices, tex_coords, faces }
}

fn indices_to_face(indices: [u32; 4]) -> ([Indices; 3], Option<Indices>) {
    let [a, b, c, d] = indices.map(|i| NonZeroU32::new(i + 1).unwrap());
    (
        [
            Indices { vertex: a, texture: None, normal: None },
            Indices { vertex: b, texture: None, normal: None },
            Indices { vertex: c, texture: None, normal: None },
        ],
        Some(Indices { vertex: d, texture: None, normal: None }),
    )
}

struct Wall {
    start: [f32; 2],
    end: [f32; 2],
    height: f32,
}
