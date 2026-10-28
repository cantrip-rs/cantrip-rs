use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::mayhem;
use cantrip_mechanics::zone_cast_from;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;


/// "Spells you cast from your graveyard cost {2} less to cast" (CR 601.2f): a spell on the stack that was cast from a
/// graveyard, or a card in one that would be.
fn graveyard_spells_cost_two_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && zone_cast_from(q, spell) == Some(ZoneKind::Graveyard) {
        CostDelta::less(2)
    } else {
        CostDelta::NONE
    }
}

pub static BACK: CardDef = CardDef {
    cost_changes: &[graveyard_spells_cost_two_less],
    granted_ways: &[mayhem::GRANTED],
    ..oracle::GREEN_GOBLIN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, effects::this_connives, "Whenever Norman Osborn deals combat damage to a player, he connives."),
        ability(cost("{1}{U}{B}{R}"), effects::transform_self, "{1}{U}{B}{R}: Transform Norman Osborn. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::NORMAN_OSBORN
};
