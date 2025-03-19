vec3 estimate_normal(vec3 p) {
    const float eps = 0.0001;
    return normalize(vec3(
        sdf_scene(vec3(p.x + eps, p.y, p.z)).x - sdf_scene(vec3(p.x - eps, p.y, p.z)).x,
        sdf_scene(vec3(p.x, p.y + eps, p.z)).x - sdf_scene(vec3(p.x, p.y - eps, p.z)).x,
        sdf_scene(vec3(p.x, p.y, p.z + eps)).x - sdf_scene(vec3(p.x, p.y, p.z - eps)).x
    ));
}

int ray_march(vec3 pos, vec3 ray_dir, inout float dist) {
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 ray_pos = pos + ray_dir * dist;
        float de = sdf_scene(ray_pos);

        dist += de * 0.5;

        // increase epsilon with number of iterations to reduce aliasing
        if (de < epsilon * sqrt(float(i)) || dist > MAX_DIST) {
            return i + 1;
        }
    }

    return MAX_STEPS;
}

vec3 calc_lightning(vec3 pos, vec3 dir, float dist, int steps, vec3 ambient_color, vec3 diffuse_color) {
    const vec3 back_pos = pos + dir * dist;

    // The (log(epsilon) * 2.0) offset is to compensate for the fact
    // that more steps are taken when epsilon is small.
    float adjusted = max(0.0, float(steps) + log(epsilon) * 2.0);
    float adjustedMax = float(MAX_STEPS) + log(epsilon) * 2.0;
    // Sqrt increases contrast.
    float distRatio = sqrt(adjusted / adjustedMax) * 0.8;

    vec3 normal = estimate_normal(back_pos);
    vec3 light_dir = normalize(ubo.light_pos.xyz);
    float lambertian = max(dot(light_dir, normal), 0.0);

    float shadow = 1.0;
    if (enable_shadows) {
        dist = 0.0;
        ray_march(back_pos, light_dir, dist);
        shadow = dist < MAX_DIST ? 0.1 : 1.0;
    }

    return ambient_color * (1 - distRatio)
        + diffuse_color * lambertian * shadow;
}
