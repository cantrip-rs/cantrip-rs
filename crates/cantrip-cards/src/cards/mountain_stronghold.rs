use cantrip_api::*;
use cantrip_mechanics::{banding, statics};
use cantrip_oracle as oracle;

/// "Red legendary creatures you control have "bands with other legendary creatures."" (CR 604.1).
fn red_legendary_creatures_control_have_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).colors.has(q.words(asker.source).color(Color::Red))
        && (*so_far).supertypes.legendary && (*so_far).is(CardType::Creature)
        && asker.controller == q.controller(affected)
}

/// The other band members must be legendary creatures (CR 702.22c).
fn granted_0_bands_0(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).supertypes.legendary && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            red_legendary_creatures_control_have_affects,
            statics::grant_delayed_ability,
            "Red legendary creatures you control have \"bands with other legendary creatures.\"",
        ),
    ],
    delayed: &[banding::with_others(granted_0_bands_0, "Bands with other legendary creatures")],
    ..oracle::MOUNTAIN_STRONGHOLD
};
