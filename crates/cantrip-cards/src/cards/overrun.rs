use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Creatures you control get +3/+3 and gain trample until end of turn." Only the creatures you control as it resolves are
/// affected, none that comes under your control later in the turn (CR 611.2c, the ruling).
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_get_and_gain::<3, 3, { Keywords::of(&[Keyword::Trample]).0 }>),
    ..oracle::OVERRUN
};
