use cantrip_api::*;
use cantrip_mechanics::triggers::PROWESS;
use cantrip_oracle as oracle;

/// "Activate only if an opponent has been dealt noncombat damage this turn" (CR 602.5b): this turn's noncombat damage to
/// an opponent of the activating player.
fn opponent_was_dealt_noncombat_damage_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    cantrip_mechanics::history::opponent_dealt_noncombat_damage_this_turn(q.history(), q.state(), asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        ability(
            cost("{B}{R}"),
            cantrip_mechanics::effects::return_this_card_with_finality,
            "{B}{R}: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only if an opponent has been dealt noncombat damage this turn.",
        )
        .from(ZoneKind::Graveyard)
        .condition(opponent_was_dealt_noncombat_damage_this_turn),
    ],
    ..oracle::GRIM_REPRISER
};
