use super::*;

pub fn vec_to_rot(v: IVec2) -> i32 {
    if v.y < 0 {
        return 0;
    }
    if v.y > 0 {
        return 2;
    }
    if v.x > 0 {
        return 1;
    }
    if v.x < 0 {
        return 0;
    }
    unreachable!()
}
