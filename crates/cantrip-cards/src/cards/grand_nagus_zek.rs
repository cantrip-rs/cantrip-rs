use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Grand Nagus Zek gets +1/+1 for each Treasure you control." "You" means the source's current controller (CR 109.5),
/// and the Treasure count uses characteristics from earlier layers in layer 7c (CR 613.4c); tokens and nontoken Treasures count.
fn plus_one_plus_one_per_treasure_you_control(
    view: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let n = lands::count_you_control(view, source, &|c| c.has_subtype(Subtype::Treasure));
    chars.power = chars.power.map(|v| v + n);
    chars.toughness = chars.toughness.map(|v| v + n);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        plus_one_plus_one_per_treasure_you_control,
        "Grand Nagus Zek gets +1/+1 for each Treasure you control.",
    )],
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_DIES,
        tokens::create_a_treasure,
        "Whenever another creature dies, create a Treasure token. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::GRAND_NAGUS_ZEK
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
