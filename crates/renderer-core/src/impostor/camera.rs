use presentation::records::Camera;
use presentation::units::skin::mul;
use presentation::view::look_at64;

pub(super) fn frame(bounds: [[f64; 3]; 2]) -> ([f64; 3], f64) {
    let center = std::array::from_fn(|c| bounds[0][c] + (bounds[1][c] - bounds[0][c]) * 0.5);
    let delta = std::array::from_fn::<_, 3, _>(|c| bounds[1][c] - bounds[0][c]);
    let radius = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2])
        .sqrt()
        .max(0.02)
        * 0.5;
    (center, radius)
}
pub(super) fn camera(center: [f64; 3], radius: f64, yaw: f64, pitch: f64) -> Camera {
    let distance = radius * 4.;
    let eye = [
        center[0] + libm::cos(pitch) * libm::cos(yaw) * distance,
        center[1] + libm::cos(pitch) * libm::sin(yaw) * distance,
        center[2] + libm::sin(pitch) * distance,
    ];
    let mut projection = [0.; 16];
    projection[0] = 1. / radius;
    projection[5] = 1. / radius;
    projection[10] = 1. / (2. * radius);
    projection[14] = (distance + radius) / (2. * radius);
    projection[15] = 1.;
    let matrix = mul(&projection, &look_at64(eye, center, [0., 0., 1.]));
    Camera {
        view_proj: matrix.map(|n| n as f32),
        eye: eye.map(|n| n as f32),
        ..Default::default()
    }
}
