use cantrip_api::*;
use cantrip_mechanics::{banding, statics};
use cantrip_oracle as oracle;

/// "Black legendary creatures you control have "bands with other legendary creatures."" (CR 604.1).
fn black_legendary_creatures_control_have_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).colors.has(q.words(asker.source).color(Color::Black))
        && (*so_far).supertypes.legendary && (*so_far).is(CardType::Creature)
        && asker.controller == q.controller(affected)
}

/// "Black legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)" (CR 702.22b).
fn granted_0_bands_0(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).supertypes.legendary && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            black_legendary_creatures_control_have_affects,
            statics::grant_delayed_ability,
            "Black legendary creatures you control have \"bands with other legendary creatures.\"",
        ),
    ],
    delayed: &[banding::with_others(granted_0_bands_0, "Bands with other legendary creatures.")],
    ..oracle::UNHOLY_CITADEL
};
