pub use glam::{Mat2, Vec2};

pub fn mat_add_scalar(m: Mat2, sc: f32) -> Mat2 {
    Mat2::from_cols_array(&m.to_cols_array().map(|v| v + sc))
}

pub fn square_vec(v: glam::Vec2) -> glam::Vec2 {
    v * v
}

pub fn polar_decomp(m: glam::Mat2) -> (glam::Mat2, glam::Mat2) {
    // coords flipped because i think the orig code is row-major
    let mc = m.to_cols_array_2d();
    let x = mc[0][0] + mc[1][1];
    let y = mc[0][1] - mc[1][0];
    // Could use Quake inv_sqrt!
    let scale = 1.0 / (x * x + y * y).sqrt();
    let c = x * scale;
    let s = y * scale;
    let R = glam::Mat2::from_cols(glam::Vec2::new(c, s), glam::Vec2::new(-s, c));
    let S = R.transpose() * m;
    (R, S)
}

#[inline(always)]
pub fn svd(m: glam::Mat2) -> (glam::Mat2, glam::Mat2, glam::Mat2) {
    let mut svd_u = glam::Mat2::ZERO;
    let mut sig = glam::Mat2::ZERO;
    let mut svd_v = glam::Mat2::ZERO;

    svd_inner(m, &mut svd_u, &mut sig, &mut svd_v);

    (svd_u, sig, svd_v)
}

pub fn svd_inner(m: glam::Mat2, u: &mut glam::Mat2, sig: &mut glam::Mat2, v: &mut glam::Mat2) {
    let (u_, S) = polar_decomp(m);
    *u = u_;

    let mut c = 0.0;
    let mut s = 0.0;

    // TODO: Might need to make this S.row(1).x
    let some_value = S.row(0).y;

    if some_value.abs() < 1e-6 {
        *sig = S;
        c = 1.0;
        s = 0.0;
    } else {
        let tao = 0.5 * (S.row(0).x - S.row(1).y);
        let w = (tao * tao + some_value * some_value).sqrt();
        let t = if tao > 0.0 {
            some_value / (tao + w)
        } else {
            some_value / (tao - w)
        };
        c = 1.0 / (t * t + 1.0).sqrt();
        s = -t * c;
        sig.col_mut(0).x = c * c * S.col(0).x - 2.0 * c * s * some_value + s * s * S.col(1).y;
        sig.col_mut(1).y = s * s * S.col(0).x + 2.0 * c * s * some_value + c * c * S.col(1).y;
    }

    if sig.col(0).x < sig.col(1).y {
        let mut old_sig = sig.to_cols_array();
        old_sig.swap(0, 3);
        *sig = glam::Mat2::from_cols_array(&old_sig);
        *v = glam::Mat2::from_cols_array(&[-s, c, -c, -s]); // TODO: should be col major
    } else {
        *v = glam::Mat2::from_cols_array(&[c, s, -s, c]); // TODO: should be col major
    }

    *v = v.transpose();
    *u = *u * *v;
}
