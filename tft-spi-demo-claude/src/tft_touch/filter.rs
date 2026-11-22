//! Turning a burst of noisy XPT2046 readings into one touch point.

/// Combines raw readings taken during one touch: readings stuck at either end
/// of the range (0 or 4095, common while the finger is landing or lifting)
/// are dropped, then each axis takes its median, which ignores the odd wild
/// reading. `None` if no usable readings are left.
pub fn median_reading(readings: &[(u16, u16)]) -> Option<(u16, u16)> {
    let usable = |v: u16| (1..4095).contains(&v);
    let (mut xs, mut ys): (Vec<u16>, Vec<u16>) = readings
        .iter()
        .copied()
        .filter(|&(x, y)| usable(x) && usable(y))
        .unzip();
    if xs.is_empty() {
        return None;
    }

    xs.sort_unstable();
    ys.sort_unstable();
    Some((xs[xs.len() / 2], ys[ys.len() / 2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_each_axis() {
        assert_eq!(median_reading(&[(100, 900), (300, 700), (200, 800)]), Some((200, 800)));
        // Axes are independent: the result needn't be one of the readings
        assert_eq!(median_reading(&[(1, 30), (2, 10), (3, 20)]), Some((2, 20)));
        // Even count: the upper of the two middle values
        assert_eq!(median_reading(&[(10, 10), (20, 20), (30, 30), (40, 40)]), Some((30, 30)));
        assert_eq!(median_reading(&[(1234, 2345)]), Some((1234, 2345)));
    }

    #[test]
    fn ignores_wild_readings() {
        let mut readings = vec![(2000, 1500); 10];
        readings.extend([(4000, 100), (50, 3900), (2001, 1501)]);
        assert_eq!(median_reading(&readings), Some((2000, 1500)));
    }

    #[test]
    fn drops_stuck_readings() {
        let readings = [(0, 1500), (2000, 4095), (4095, 0), (2000, 1500), (0, 0)];
        assert_eq!(median_reading(&readings), Some((2000, 1500)));
        assert_eq!(median_reading(&[(0, 0), (4095, 4095), (0, 1000)]), None);
        // 1 and 4094 are real readings
        assert_eq!(median_reading(&[(1, 4094)]), Some((1, 4094)));
    }

    #[test]
    fn no_readings() {
        assert_eq!(median_reading(&[]), None);
    }
}
