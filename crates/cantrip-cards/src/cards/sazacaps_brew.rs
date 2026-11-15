use cantrip_api::*;
use cantrip_mechanics::{effects, gift, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Target player draws two cards. If the gift was promised, target creature you control gets +2/+0 until end of turn." (CR 702.174,
/// 608.2b, 611.2a): the gift was given before this runs (CR 702.174j). The creature is a target only when the gift was promised (CR 601.2c,
/// `TargetSpec::if_promised`), so without a promise there is none to pump. Each target is judged as the spell resolves: a player who is no
/// longer a legal target draws nothing and the creature still gets the pump, and the other way round.
fn brew(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    // A sentence ends (CR 608.2c): the state triggers judge what it did before the next (`Game::end_sentence`).
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    pump(g, item.object, creature, 2, 0);
    Ok(())
}

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, targets::CREATURE_YOU_CONTROL.between(0, 0).if_promised(1, 1)], brew),
    gift: Some(gift::TAPPED_FISH),
    // "As an additional cost to cast this spell, discard a card" (CR 601.2b, 118.8): exactly one, paid with the other costs.
    additional_cost: Some(discard_cards(1)),
    ..oracle::SAZACAPS_BREW
};
