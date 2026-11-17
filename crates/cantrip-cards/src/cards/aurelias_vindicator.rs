use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_LEAVES, THIS_TURNED_FACE_UP};
use cantrip_mechanics::{disguise, effects, targets};
use cantrip_oracle as oracle;

/// "Other target creatures from the battlefield and/or creature cards from graveyards" (CR 115.1, 109.5): a creature on the
/// battlefield other than this one, or a creature card in any graveyard.
fn other_creature_or_creature_card_in_a_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::other_creature_filter(q, asker, t) || targets::card_in_graveyard(q, t, &[CardType::Creature])
}

/// X in the abilities of a permanent turned face up by a disguise cost with an {X} is the X chosen as the special action was
/// taken (CR 702.168e, 107.3d), the one the engine keeps on the permanent: the most targets the ability may have.
fn x_chosen_for_the_disguise_cost(q: &dyn Query, asker: &Asker) -> u32 {
    q.obj(asker.source).x as u32
}

/// "Exile up to X other target creatures from the battlefield and/or creature cards from graveyards" (CR 607.2a, 608.2b): the
/// targets still legal are exiled together and remembered as exiled with this creature, for its leaves ability. They stay
/// exiled when this creature has already left (the ruling: that ability found nothing to return).
fn exile_the_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = targets::legal_objects(g, item);
    if !chosen.is_empty() {
        g.exile_with(&chosen, g.linked(item), false);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{X}{3}{W}")), "Disguise {X}{3}{W}")],
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(
            THIS_TURNED_FACE_UP,
            exile_the_targets,
            "When this creature is turned face up, exile up to X other target creatures from the battlefield and/or creature cards from graveyards.",
        )
        .targets(&[target(other_creature_or_creature_card_in_a_graveyard).between(0, TargetSpec::COUNTED)])
        .counted_by(x_chosen_for_the_disguise_cost),
        triggered(
            THIS_LEAVES,
            effects::return_exiled_to_hand,
            "When this creature leaves the battlefield, return the exiled cards to their owners' hands.",
        ),
    ],
    ..oracle::AURELIAS_VINDICATOR
};
