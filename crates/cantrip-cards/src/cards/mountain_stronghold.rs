use cantrip_api::*;
use cantrip_mechanics::{banding, statics};
use cantrip_oracle as oracle;

/// "Red legendary creatures you control" (CR 604.1, 109.2).
fn red_legendary_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_creatures(q, source, affected, so_far)
        && so_far.supertypes.legendary
        && so_far.colors.has(q.words(source).color(Color::Red))
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            red_legendary_creatures_you_control,
            statics::grant_delayed_ability,
            "Red legendary creatures you control have \"bands with other legendary creatures.\"",
        ),
    ],
    delayed: &[banding::WITH_OTHER_LEGENDARY_CREATURES],
    ..oracle::MOUNTAIN_STRONGHOLD
};
