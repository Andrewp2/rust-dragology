mod frobenius;
mod zx;
use super::*;
pub fn entries() -> Vec<Entry> {
    let mut a = frobenius::entries();
    a.extend(zx::entries());
    a
}
