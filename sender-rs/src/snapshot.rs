//! Source-independent view of the game state, handed from a source to the runner.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkState {
    #[default]
    NoGame,
    Connected,
    InVehicle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceKind {
    #[default]
    Sc,
    Random,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleSnap {
    pub name: String,
    /// Raw (unscaled); None if the shields message is absent.
    pub shield_level: Option<f32>,
    /// Raw.
    pub shield_faces: Vec<f32>,
    /// Raw; None if the instruments message is absent.
    pub hydrogen_fuel: Option<f32>,
    pub quantum_fuel: Option<f32>,
    /// Raw enum values in ANN_* order; None if the annunciators message is absent.
    pub annunciators: Option<[i32; 10]>,
    /// None if flight.state is absent.
    pub is_landed: Option<bool>,
    /// None if flight.state is absent OR gear is unset (proto3 optional).
    pub gear: Option<i32>,
    /// None if flight.settings.ifcs_coupled is absent.
    pub decoupled: Option<bool>,
    /// None if master_mode is absent.
    pub master_mode: Option<i32>,
    pub qt_spooled: Option<bool>,
    /// Always Some for the SC source (scalar on VehicleState).
    pub lights: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorSnap {
    /// 0..1
    pub health: Option<f32>,
    /// 0..1
    pub oxygen: Option<f32>,
    pub locomotion: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub link: LinkState,
    pub source: SourceKind,
    pub vehicle: Option<VehicleSnap>,
    pub actor: Option<ActorSnap>,
}

impl Snapshot {
    pub fn no_game(source: SourceKind) -> Self {
        Self { link: LinkState::NoGame, source, vehicle: None, actor: None }
    }
}
