use std::{f32::consts::TAU, time::Duration};

use crownui::prelude::{Cx, Signal};

const SHAKE: Duration = Duration::from_millis(420);
const SHAKE_DISTANCE: f32 = 10.0;
const SHAKE_SWINGS: f32 = 3.0;

/// The lock screen's wrong-password shake: a decaying sideways swing written into `offset`
/// every frame until it settles at zero.
pub fn shake(cx: &mut Cx, offset: Signal<f32>) {
    cx.request_animation_frame(move |cx, start| swing(cx, offset, start, start));
}

fn swing(cx: &mut Cx, offset: Signal<f32>, start: Duration, now: Duration) {
    let elapsed = now.saturating_sub(start);
    offset.set(cx, shake_offset(elapsed));
    if elapsed < SHAKE {
        cx.request_animation_frame(move |cx, now| swing(cx, offset, start, now));
    }
}

/// How far the card sits off center `elapsed` into a shake.
pub fn shake_offset(elapsed: Duration) -> f32 {
    let progress = elapsed.as_secs_f32() / SHAKE.as_secs_f32();
    if progress >= 1.0 {
        return 0.0;
    }
    SHAKE_DISTANCE * (1.0 - progress) * (progress * SHAKE_SWINGS * TAU).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swings_both_ways_and_settles() {
        let samples: Vec<f32> = (0..=42)
            .map(|step| shake_offset(Duration::from_millis(step * 10)))
            .collect();
        assert!(samples.iter().any(|offset| *offset > 3.0));
        assert!(samples.iter().any(|offset| *offset < -3.0));
        assert!(samples.iter().all(|offset| offset.abs() <= SHAKE_DISTANCE));
        assert_eq!(shake_offset(SHAKE), 0.0);
    }
}
