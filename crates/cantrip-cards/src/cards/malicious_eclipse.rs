use cantrip_api::*;
use cantrip_mechanics::effects::pump_all_creatures;
use cantrip_mechanics::replacements::{exile_instead_apply_granted, opponents_creature_would_die};
use cantrip_oracle as oracle;

/// "If a creature an opponent controls would die this turn" (CR 614.1a, 700.4): read for every creature then, whenever it
/// would, not only the ones on the battlefield as the spell resolves (the ruling). "Opponent" is the spell's controller's.
fn would_die(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    opponents_creature_would_die(q, asker, proposal)
}

/// "All creatures get -2/-2 until end of turn. If a creature an opponent controls would die this turn, exile it instead."
/// The -2/-2 goes to the creatures there as it resolves (CR 611.2c); the replacement is about the game, not any creature, for
/// the turn (CR 611.2), made by the resolving spell for its controller.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_all_creatures(g, item.object, -2, -2);
    g.end_sentence();
    g.add_player_effect(item.controller, item.object, EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    granted_replacements: &[granted_replacement(ReplacementKind::ZoneChange, would_die, exile_instead_apply_granted, "exiled instead of dying")
        .text("If a creature an opponent controls would die this turn, exile it instead.")],
    ..oracle::MALICIOUS_ECLIPSE
};
