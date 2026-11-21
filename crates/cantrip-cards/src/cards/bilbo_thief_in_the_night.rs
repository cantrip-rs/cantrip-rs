use cantrip_api::*;
use cantrip_mechanics::{choose, triggers, zone_cast_from};
use cantrip_oracle as oracle;

/// Spells you cast from anywhere other than your hand cost {1} less to cast (CR 601.2f): the zone a spell is in as its
/// cost is worked out, or the one it was cast from once it is on the stack.
fn outside_hand_costs_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) {
        return CostDelta::NONE;
    }
    if zone_cast_from(q, spell).is_some_and(|z| z != ZoneKind::Hand) { CostDelta::less(1) } else { CostDelta::NONE }
}

/// Whenever Bilbo attacks, you may cast an artifact, instant, or sorcery spell from your graveyard. If an instant or
/// sorcery spell cast this way would be put into your graveyard, exile it instead. Nothing is asked when no card there
/// can be cast.
fn recast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let graveyard = g.state().zone(Zone::graveyard(you)).to_vec();
    // "An artifact, instant, or sorcery spell": the spell the card would be cast as is judged, with every effect that would apply to
    // it on the stack (CR 601.3e, `Game::chars_as_cast`).
    choose::cast_up_to_among(
        g,
        you,
        item.object,
        Purpose::Cast,
        &graveyard,
        1,
        CastTerms { exile_instead: ExileInstead::InstantOrSorcery, ..CastTerms::default() },
        |g, card, w| {
            let spell = g.chars_as_cast(card, w.way, you);
            spell.is(CardType::Artifact) || spell.is(CardType::Instant) || spell.is(CardType::Sorcery)
        },
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, recast, "Whenever Bilbo attacks, you may cast an artifact, instant, or sorcery spell from your graveyard. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.")],
    cost_changes: &[outside_hand_costs_one_less],
    ..oracle::BILBO_THIEF_IN_THE_NIGHT
};
