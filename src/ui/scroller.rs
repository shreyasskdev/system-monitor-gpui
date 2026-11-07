use gpui_kit::*;
use std::time::{Duration, Instant};

/// iOS-style inertial + rubber-band spring scroll state
pub struct BouncyScroller {
    pub handle: ScrollHandle,
    /// Current rubber-band overscroll displacement in pixels (+ when pulled past top, - when pulled past bottom)
    pub overscroll: f32,
    /// Current spring/inertia velocity
    pub velocity: f32,
    pub last_wheel_input: Instant,
}

impl BouncyScroller {
    pub fn new() -> Self {
        Self {
            handle: ScrollHandle::new(),
            overscroll: 0.0,
            velocity: 0.0,
            last_wheel_input: Instant::now() - Duration::from_secs(10),
        }
    }

    pub fn on_wheel(&mut self, dy: f32) {
        self.last_wheel_input = Instant::now();
        let current_y = f32::from(self.handle.offset().y);
        let max_y = f32::from(self.handle.max_offset().y).abs();

        // In GPUI, scroll offset.y is 0.0 at the top and negative (-max_y) at the bottom.
        let proposed_y = current_y + dy;

        if proposed_y > 0.0 {
            // Pulling past the TOP edge -> rubber-band resistance
            self.handle.set_offset(point(px(0.0), px(0.0)));
            let resistance = 1.0 / (1.0 + self.overscroll.abs() / 140.0);
            self.overscroll = (self.overscroll + dy * 0.55 * resistance).clamp(-160.0, 160.0);
            self.velocity = 0.0;
        } else if proposed_y < -max_y {
            // Pulling past the BOTTOM edge -> rubber-band resistance
            self.handle.set_offset(point(px(0.0), px(-max_y)));
            let resistance = 1.0 / (1.0 + self.overscroll.abs() / 140.0);
            self.overscroll = (self.overscroll + dy * 0.55 * resistance).clamp(-160.0, 160.0);
            self.velocity = 0.0;
        } else if self.overscroll.abs() > 0.5 {
            // Unwind overscroll first before moving the real scroll offset
            self.overscroll += dy * 0.65;
        } else {
            self.overscroll = 0.0;
            self.handle
                .set_offset(point(px(0.0), px(proposed_y.clamp(-max_y, 0.0))));
            // Feed smooth kinetic momentum
            self.velocity = (self.velocity * 0.55 + dy * 26.0).clamp(-3200.0, 3200.0);
        }
    }

    /// Advances the iOS spring & momentum physics by `dt` seconds. Returns `true` if still moving.
    pub fn step_physics(&mut self, dt: f32) -> bool {
        let mut active = false;
        let max_y = f32::from(self.handle.max_offset().y).abs();
        let mut current_y = f32::from(self.handle.offset().y);

        // 1. Apply kinetic inertia if not currently in overscroll
        if self.velocity.abs() > 8.0 && self.overscroll.abs() < 0.5 {
            active = true;
            current_y += self.velocity * dt;
            // Smooth iOS deceleration friction
            self.velocity *= (-7.5 * dt).exp();

            if current_y > 0.0 {
                self.overscroll += current_y * 0.45;
                current_y = 0.0;
                self.velocity *= 0.25;
            } else if current_y < -max_y {
                self.overscroll += (current_y + max_y) * 0.45;
                current_y = -max_y;
                self.velocity *= 0.25;
            }
            self.handle
                .set_offset(point(px(0.0), px(current_y.clamp(-max_y, 0.0))));
        } else if self.velocity.abs() <= 8.0 {
            self.velocity = 0.0;
        }

        // 2. Critically-damped Hooke's law spring bounce-back for rubber-band overscroll
        let ms_since_wheel = self.last_wheel_input.elapsed().as_millis();
        if self.overscroll.abs() > 0.25 {
            active = true;
            if ms_since_wheel > 45 {
                let stiffness = 210.0;
                let damping = 24.0;
                let spring_accel = -stiffness * self.overscroll - damping * self.velocity;
                self.velocity += spring_accel * dt;
                self.overscroll += self.velocity * dt;

                if self.overscroll.abs() < 0.35 && self.velocity.abs() < 6.0 {
                    self.overscroll = 0.0;
                    self.velocity = 0.0;
                }
            }
        } else {
            self.overscroll = 0.0;
        }

        active
    }

    /// Applies visual rubber-band bounce padding at the top or bottom of a scrolled list
    pub fn apply_rubber_band(&self, container: Div) -> Div {
        if self.overscroll > 0.5 {
            container.pt(px(self.overscroll))
        } else if self.overscroll < -0.5 {
            container.pb(px(-self.overscroll))
        } else {
            container
        }
    }
}
