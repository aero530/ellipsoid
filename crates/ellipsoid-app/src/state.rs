//! Application state and the recompute pipeline.
//!
//! Only [`AppState::input`] is authoritative. Everything in [`Derived`] is a
//! pure function of it and is rebuilt whenever it changes — the whole pipeline
//! is sub-millisecond at realistic division counts, so nothing is cached.
//!
//! This replaces the three Redux slices (`input`, `geometry`, `edges`), two of
//! which stored derived data as if it were independent truth.

use bevy::prelude::*;
use ellipsoid_core::input::ValidationError;
use ellipsoid_core::{
    Cutout, EllipsoidInput, FlatGeometry, Geometry, compute_flat_geometry, compute_geometry,
};
use ellipsoid_pattern::{Scene, build_scene};

use crate::platform;

/// Everything derived from the current input.
pub struct Derived {
    pub geometry: Geometry,
    pub flat: FlatGeometry,
    pub scene: Scene,
}

/// A transient message for the status bar.
pub struct Status {
    pub text: String,
    pub is_error: bool,
}

impl Status {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}

/// What a pointer drag has hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grab {
    /// A whole cutout, by index.
    Cutout(usize),
    /// One vertex of a polygon: `(cutout, vertex)`.
    Vertex(usize, usize),
}

/// Pan and zoom for the 2D pattern preview.
#[derive(Clone, Copy, Default)]
pub struct PreviewView {
    /// Screen-space offset applied after centring.
    pub pan: Vec2,
    pub zoom: f32,
    /// While false the view refits every frame.
    ///
    /// Fitting once and latching is tempting but fragile: panel sizes settle
    /// over the first frames, so an early fit locks in the wrong zoom, and a
    /// window resize invalidates it anyway. Auto-fitting until the user pans or
    /// zooms is both simpler and better behaved — and the Fit button just hands
    /// control back.
    pub user_adjusted: bool,
}

#[derive(Resource)]
pub struct AppState {
    /// The only authoritative state.
    pub input: EllipsoidInput,
    /// `None` while the input is invalid.
    pub derived: Option<Derived>,
    /// Why `derived` is `None`, if it is.
    pub problems: Vec<ValidationError>,
    /// Set whenever `input` changes; consumed by [`recompute`].
    pub dirty: bool,
    /// Bumped every time `derived` is rebuilt.
    ///
    /// Bevy's own change detection is too coarse here: it fires for any touch
    /// of the resource, including a status message, which would rebuild both
    /// 3D meshes on every save. Consumers track the value they last saw.
    pub geometry_generation: u64,
    /// Diameter given to the next cutout placed, in the document's unit.
    pub new_cutout_diameter: f64,
    /// What the pointer is dragging, and where it last had hold of it.
    ///
    /// The grab point is tracked in surface coordinates and updated every
    /// frame, so a drag moves by the delta rather than snapping to the cursor.
    pub drag: Option<(Grab, f64, f64)>,
    /// Vertices placed so far for a shape being drawn, if any.
    pub draft: Option<Vec<[f64; 2]>>,
    /// The shape whose individual points are being edited, if any.
    ///
    /// Editing is a mode rather than a modifier because a polygon's vertices
    /// sit right on top of its outline: without one, "grab the shape" and
    /// "grab a point of the shape" would be the same gesture.
    pub editing: Option<usize>,
    /// Set when the cutout list changes without the geometry changing, so the
    /// 3D markers rebuild.
    pub cutouts_dirty: bool,
    /// Where a settings file the user is choosing will be delivered.
    pub inbox: platform::Inbox,
    pub status: Option<Status>,
    pub view: PreviewView,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input: EllipsoidInput::default(),
            derived: None,
            problems: Vec::new(),
            // Compute on the first frame so there is something to look at.
            dirty: true,
            geometry_generation: 0,
            new_cutout_diameter: Cutout::default_diameter(EllipsoidInput::default().unit),
            drag: None,
            draft: None,
            editing: None,
            cutouts_dirty: false,
            inbox: platform::Inbox::default(),
            status: None,
            view: PreviewView::default(),
        }
    }
}

impl AppState {
    /// Record that the input changed and the pattern needs rebuilding.
    pub fn touch(&mut self) {
        self.dirty = true;
    }

    /// Suggested filename for an export, without extension.
    pub fn filename_stem(&self) -> String {
        self.input.filename_stem()
    }
}

/// Rebuild [`Derived`] when the input has changed.
///
/// Validation runs first: the core geometry functions are a faithful port and
/// would silently coerce bad values rather than complain, so the UI checks
/// before calling them and shows the reasons instead of a garbage pattern.
pub fn recompute(mut state: ResMut<AppState>) {
    if !state.dirty {
        return;
    }
    state.dirty = false;

    match state.input.validate() {
        Err(problems) => {
            state.problems = problems;
            state.derived = None;
        }
        Ok(()) => {
            state.problems.clear();
            let geometry = compute_geometry(&state.input);
            let flat = compute_flat_geometry(&geometry, &state.input);
            let scene = build_scene(&state.input, &geometry, &flat);
            state.derived = Some(Derived {
                geometry,
                flat,
                scene,
            });
            state.geometry_generation = state.geometry_generation.wrapping_add(1);
        }
    }
}

/// Apply a settings file once the picker has delivered it.
///
/// Runs before [`recompute`], so a file chosen on one frame is drawn on the
/// same frame it arrives rather than the next.
pub fn apply_opened(mut state: ResMut<AppState>) {
    let Some(result) = state.inbox.take() else {
        return;
    };
    let opened = match result {
        Ok(platform::Opened::Cancelled) => return,
        Ok(opened) => opened,
        Err(why) => {
            state.status = Some(Status::error(format!("Could not open settings: {why}")));
            return;
        }
    };
    let platform::Opened::File { name, text } = opened else {
        return;
    };

    match EllipsoidInput::from_json(&text) {
        Ok(input) => {
            state.input = input;
            state.new_cutout_diameter = Cutout::default_diameter(state.input.unit);
            // Whatever was being edited belongs to the settings just replaced.
            state.editing = None;
            state.draft = None;
            state.drag = None;
            state.view.user_adjusted = false;
            state.status = Some(Status::info(format!("Loaded {name}")));
            state.cutouts_dirty = true;
            state.touch();
        }
        Err(e) => state.status = Some(Status::error(format!("{name} is not valid settings: {e}"))),
    }
}

/// How long the input must sit still before it is written back.
///
/// Long enough that dragging a cutout does not write once per frame, short
/// enough that closing the window straight after an edit still keeps it.
const AUTOSAVE_IDLE: f32 = 0.75;

/// Settings carried between sessions.
///
/// Kept out of [`AppState`] because none of it is input: it is bookkeeping
/// about what has already been written.
#[derive(Resource, Default)]
pub struct Persistence {
    /// The last input written out, so an unchanged one is not rewritten.
    saved: Option<EllipsoidInput>,
    /// The input as of last frame, so *changing* resets the idle timer.
    seen: Option<EllipsoidInput>,
    /// Seconds the input has been unchanged.
    idle: f32,
}

impl Persistence {
    /// Take `input` to be what is already stored, so it is not written back.
    fn assume_stored(&mut self, input: &EllipsoidInput) {
        self.saved = Some(input.clone());
        self.seen = self.saved.clone();
        self.idle = 0.0;
    }

    /// Advance the debounce for `input`, and say whether it is time to write.
    ///
    /// Split out from the system so the timing can be tested without a Bevy
    /// world or a real config directory — everything that can go subtly wrong
    /// here (writing every frame of a drag, never writing at all, writing an
    /// input that is already on disk) lives in these few lines.
    fn due(&mut self, input: &EllipsoidInput, delta: f32) -> bool {
        if self.seen.as_ref() != Some(input) {
            // Still moving. Note where it got to and start the clock again.
            self.seen = Some(input.clone());
            self.idle = 0.0;
            return false;
        }
        if self.saved == self.seen {
            return false;
        }
        self.idle += delta;
        self.idle >= AUTOSAVE_IDLE
    }
}

/// Restore last session's settings, if there are any and they still parse.
///
/// A stored file that no longer loads is reported and ignored, never fatal:
/// refusing to start because of a stale config would be the worst possible
/// outcome of a feature meant to be a convenience.
pub fn restore(mut state: ResMut<AppState>, mut persistence: ResMut<Persistence>) {
    if let Some(text) = platform::recall() {
        match EllipsoidInput::from_json(&text) {
            Ok(input) => {
                state.input = input;
                state.new_cutout_diameter = Cutout::default_diameter(state.input.unit);
                state.status = Some(Status::info(match platform::remembered_location() {
                    Some(where_) => format!("Restored your last settings from {where_}"),
                    None => "Restored your last settings".into(),
                }));
                state.touch();
            }
            Err(e) => {
                state.status = Some(Status::error(format!("Ignoring stored settings: {e}")));
            }
        }
    }

    // Unconditionally, including when there was nothing to recall: leaving
    // this unset would make the defaults look like an unsaved edit, and merely
    // opening the app would write a settings file nobody asked for.
    let input = state.input.clone();
    persistence.assume_stored(&input);
}

/// Write the settings back once the user has stopped changing them.
pub fn autosave(
    time: Res<Time>,
    mut state: ResMut<AppState>,
    mut persistence: ResMut<Persistence>,
) {
    if persistence.due(&state.input, time.delta_secs()) {
        flush(&mut state, &mut persistence);
    }
}

/// Write the settings back now, whether or not the input has settled.
///
/// Called on the way out: an edit made in the last fraction of a second has not
/// tripped the idle timer yet, and would otherwise be the one thing lost.
///
/// Desktop only in practice — closing a browser tab raises no `AppExit`, so on
/// the web [`autosave`]'s idle write is the whole story.
pub fn autosave_on_exit(
    // 0.19 renamed Bevy's buffered events to messages; `AppExit` is one.
    mut exits: MessageReader<AppExit>,
    mut state: ResMut<AppState>,
    mut persistence: ResMut<Persistence>,
) {
    if exits.read().next().is_none() || persistence.saved.as_ref() == Some(&state.input) {
        return;
    }
    flush(&mut state, &mut persistence);
}

fn flush(state: &mut AppState, persistence: &mut Persistence) {
    match platform::remember(&state.input.to_json()) {
        Ok(()) => persistence.saved = Some(state.input.clone()),
        // Report once and stop trying, rather than failing every frame: mark it
        // saved so only the *next* edit tries again.
        Err(e) => {
            state.status = Some(Status::error(format!("Could not remember settings: {e}")));
            persistence.saved = Some(state.input.clone());
        }
    }
    persistence.idle = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame at 60 Hz.
    const FRAME: f32 = 1.0 / 60.0;

    /// Run `frames` frames without touching the input, doing what [`flush`]
    /// would whenever the debounce asks, and count the writes.
    fn settle(p: &mut Persistence, input: &EllipsoidInput, frames: usize) -> usize {
        let mut writes = 0;
        for _ in 0..frames {
            if p.due(input, FRAME) {
                writes += 1;
                p.saved = Some(input.clone());
                p.idle = 0.0;
            }
        }
        writes
    }

    /// A [`Persistence`] set up the way `restore` leaves it — including on a
    /// first run, where there was nothing to recall and the defaults stand in
    /// for what is stored.
    fn started(input: &EllipsoidInput) -> Persistence {
        let mut p = Persistence::default();
        p.assume_stored(input);
        p
    }

    #[test]
    fn merely_opening_the_app_writes_nothing() {
        // Including the first run ever: the defaults are not an edit, and a
        // settings file appearing because the app was opened is a surprise.
        let input = EllipsoidInput::default();
        assert_eq!(settle(&mut started(&input), &input, 300), 0);
    }

    #[test]
    fn a_change_is_written_once_after_it_settles() {
        let mut input = EllipsoidInput::default();
        let mut p = started(&input);

        input.a = 4.0;
        // Well short of the idle time: still being edited.
        assert_eq!(settle(&mut p, &input, 10), 0);
        // And then exactly one write, not one per frame for ever after.
        assert_eq!(settle(&mut p, &input, 300), 1);
    }

    #[test]
    fn a_continuing_drag_never_reaches_the_timer() {
        // The case the debounce exists for: a cutout dragged for two seconds
        // changes the input every frame and must not write on any of them.
        let mut input = EllipsoidInput::default();
        input.cutouts.push(Cutout::hole(0.1, 0.5, 0.1));
        let mut p = started(&input);

        let mut writes = 0;
        for _ in 0..120 {
            input.cutouts[0].translate(0.001, 0.0);
            if p.due(&input, FRAME) {
                writes += 1;
            }
        }
        assert_eq!(writes, 0, "wrote while the pointer was still moving");

        // Letting go writes once the input has been still long enough.
        assert_eq!(settle(&mut p, &input, 300), 1);
    }

    #[test]
    fn returning_to_the_saved_state_cancels_the_write() {
        // Change something and change it back: the file already says this, so
        // there is nothing to do.
        let original = EllipsoidInput::default();
        let mut p = started(&original);
        let edited = EllipsoidInput {
            a: 9.0,
            ..original.clone()
        };
        assert_eq!(settle(&mut p, &edited, 10), 0);
        assert_eq!(settle(&mut p, &original, 300), 0);
    }
}
