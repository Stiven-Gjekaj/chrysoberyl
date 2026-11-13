//! The state of the viewer, without a window.
//!
//! Each input from a person becomes one `Action`. `ViewState::apply`
//! changes the state for that action. The window only reads the state.
//! This keeps the logic testable without a GPU or a display.

/// How the window shows a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The base on the left, the candidate on the right.
    SideBySide,
    /// One image above the other, with a blend between the two.
    Overlay,
    /// The base in grey, with each changed pixel in red.
    Difference,
}

impl Mode {
    /// The number that the shader reads for this mode.
    pub fn code(self) -> u32 {
        match self {
            Mode::SideBySide => 0,
            Mode::Overlay => 1,
            Mode::Difference => 2,
        }
    }

    /// The name that the title bar shows.
    pub fn name(self) -> &'static str {
        match self {
            Mode::SideBySide => "side by side",
            Mode::Overlay => "overlay",
            Mode::Difference => "difference",
        }
    }
}

/// The decision of a person about the pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The person accepts the candidate as the new baseline.
    Accept,
    /// The person rejects the candidate.
    Reject,
}

/// One input from a person.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Show the pair in this mode.
    SetMode(Mode),
    /// Go to the next frame of a sequence.
    NextFrame,
    /// Go to the previous frame of a sequence.
    PreviousFrame,
    /// Move the image by this many window pixels.
    Pan(f32, f32),
    /// Multiply the zoom by `factor`, and keep the image point under
    /// `(x, y)` in place. `x` and `y` are relative to the pane centre.
    ZoomAt {
        /// The zoom factor.
        factor: f32,
        /// The horizontal position, relative to the pane centre.
        x: f32,
        /// The vertical position, relative to the pane centre.
        y: f32,
    },
    /// Fit the image in the pane.
    Fit,
    /// Change the overlay blend by this amount.
    Blend(f32),
    /// Show the base, then the candidate, then the base again.
    Flip,
    /// Record a decision.
    Decide(Decision),
}

/// The smallest zoom.
pub const MIN_ZOOM: f32 = 1.0 / 64.0;
/// The largest zoom.
pub const MAX_ZOOM: f32 = 256.0;

/// The state of the viewer.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// The current mode.
    pub mode: Mode,
    /// The index of the frame on view.
    pub frame: usize,
    /// The number of frames in the pair.
    pub frame_count: usize,
    /// Window pixels for one image pixel.
    pub zoom: f32,
    /// The offset of the image centre from the pane centre, in window pixels.
    pub pan: (f32, f32),
    /// The overlay blend: 0 shows the base, 1 shows the candidate.
    pub blend: f32,
    /// The last decision, if a person made one.
    pub decision: Option<Decision>,
}

impl ViewState {
    /// Make the state for a pair of `frame_count` frames.
    pub fn new(frame_count: usize) -> Self {
        ViewState {
            mode: Mode::SideBySide,
            frame: 0,
            frame_count,
            zoom: 1.0,
            pan: (0.0, 0.0),
            blend: 0.5,
            decision: None,
        }
    }

    /// Set the zoom so that an image of `image` size fits a pane of
    /// `pane` size, and put the image in the centre.
    pub fn fit(&mut self, image: (u32, u32), pane: (f32, f32)) {
        let (iw, ih) = (image.0.max(1) as f32, image.1.max(1) as f32);
        self.zoom = (pane.0 / iw).min(pane.1 / ih).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = (0.0, 0.0);
    }

    /// Change the state for one action.
    ///
    /// `Fit` needs the image and the pane size, so the window calls `fit`
    /// for it. This function ignores `Fit`.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::SetMode(mode) => self.mode = mode,
            Action::NextFrame => {
                if self.frame + 1 < self.frame_count {
                    self.frame += 1;
                }
            }
            Action::PreviousFrame => self.frame = self.frame.saturating_sub(1),
            Action::Pan(dx, dy) => {
                self.pan.0 += dx;
                self.pan.1 += dy;
            }
            Action::ZoomAt { factor, x, y } => {
                let zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                let applied = zoom / self.zoom;
                // The image point under (x, y) is (x - pan) / zoom. Keep it.
                self.pan.0 = x - (x - self.pan.0) * applied;
                self.pan.1 = y - (y - self.pan.1) * applied;
                self.zoom = zoom;
            }
            Action::Fit => {}
            Action::Blend(amount) => self.blend = (self.blend + amount).clamp(0.0, 1.0),
            Action::Flip => self.blend = if self.blend < 0.5 { 1.0 } else { 0.0 },
            Action::Decide(decision) => self.decision = Some(decision),
        }
    }

    /// Map a window position, relative to the pane centre, to an image
    /// position, for an image of `image` size.
    pub fn to_image(&self, x: f32, y: f32, image: (u32, u32)) -> (f32, f32) {
        (
            (x - self.pan.0) / self.zoom + image.0 as f32 / 2.0,
            (y - self.pan.1) / self.zoom + image.1 as f32 / 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_steps_stay_inside_the_sequence() {
        let mut state = ViewState::new(3);
        state.apply(Action::PreviousFrame);
        assert_eq!(state.frame, 0);
        for _ in 0..5 {
            state.apply(Action::NextFrame);
        }
        assert_eq!(state.frame, 2);
    }

    #[test]
    fn a_zoom_keeps_the_point_under_the_cursor_in_place() {
        let mut state = ViewState::new(1);
        state.apply(Action::Pan(13.0, -7.0));
        let before = state.to_image(40.0, 25.0, (200, 100));
        state.apply(Action::ZoomAt {
            factor: 3.0,
            x: 40.0,
            y: 25.0,
        });
        let after = state.to_image(40.0, 25.0, (200, 100));
        assert!((before.0 - after.0).abs() < 1e-3, "{before:?} {after:?}");
        assert!((before.1 - after.1).abs() < 1e-3, "{before:?} {after:?}");
        assert_eq!(state.zoom, 3.0);
    }

    #[test]
    fn the_zoom_stays_inside_its_limits() {
        let mut state = ViewState::new(1);
        for _ in 0..40 {
            state.apply(Action::ZoomAt {
                factor: 2.0,
                x: 0.0,
                y: 0.0,
            });
        }
        assert_eq!(state.zoom, MAX_ZOOM);
    }

    #[test]
    fn fit_makes_the_longer_side_fill_the_pane() {
        let mut state = ViewState::new(1);
        state.apply(Action::Pan(50.0, 50.0));
        state.fit((400, 100), (200.0, 200.0));
        assert_eq!(state.zoom, 0.5);
        assert_eq!(state.pan, (0.0, 0.0));
    }

    #[test]
    fn flip_shows_one_side_then_the_other() {
        let mut state = ViewState::new(1);
        // The blend starts at 0.5, so the first flip shows the base.
        state.apply(Action::Flip);
        assert_eq!(state.blend, 0.0);
        state.apply(Action::Flip);
        assert_eq!(state.blend, 1.0);
        state.apply(Action::Blend(5.0));
        assert_eq!(state.blend, 1.0);
    }

    #[test]
    fn the_last_decision_is_kept() {
        let mut state = ViewState::new(1);
        state.apply(Action::Decide(Decision::Reject));
        state.apply(Action::Decide(Decision::Accept));
        assert_eq!(state.decision, Some(Decision::Accept));
    }
}
