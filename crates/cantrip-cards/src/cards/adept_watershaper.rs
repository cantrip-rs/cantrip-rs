use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Other tapped creatures you control": a creature you control other than this one that is tapped now (CR 613.1f).
fn other_tapped_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::other_creatures_you_control(q, source, affected, so_far) && q.state().obj(affected).tapped
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        other_tapped_creatures_you_control,
        statics::have_indestructible,
        "Other tapped creatures you control have indestructible.",
    )],
    ..oracle::ADEPT_WATERSHAPER
};
