//! Walk-In Closet // Forgotten Cellar, a Room (CR 709.5): each door is only what its text does; the Room has the abilities
//! and play rules of its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{replacements, triggers};
use cantrip_oracle as oracle;

/// Walk-In Closet: "You may play lands from your graveyard" (CR 305.1), while this door is unlocked.
pub static WALK_IN_CLOSET: CardDef = CardDef { play_rules: &[PlayRule::LandsFromGraveyard], ..oracle::WALK_IN_CLOSET };

/// "If a card would be put into your graveyard from anywhere this turn, exile it instead" (CR 614.1a): a card, so not a
/// token, headed for the graveyard of the door's controller.
fn exile_instead_applies(_: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.is_card && z.to == Zone::graveyard(asker.controller))
}

/// "You may cast spells from your graveyard this turn, and if a card would be put into your graveyard from anywhere this
/// turn, exile it instead": two effects of the resolved ability lasting the turn (CR 611.2), about the player, so they
/// outlive the Room. Lands are not played by the first (`EffectKind::CastFromGraveyard`, CR 305.9).
fn cast_from_graveyard_and_exile_instead(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::CastFromGraveyard, Duration::EndOfTurn);
    // Its first instruction ends at the "and" (CR 608.2c): the state triggers judge it before the second (`Game::end_sentence`).
    g.end_sentence();
    g.add_effect(item.object, item.object, EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static FORGOTTEN_CELLAR: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        cast_from_graveyard_and_exile_instead,
        "When you unlock this door, you may cast spells from your graveyard this turn, and if a card would be put into your graveyard from anywhere this turn, exile it instead.",
    )],
    granted_replacements: &[granted_replacement(ReplacementKind::ZoneChange, exile_instead_applies, replacements::exile_instead_apply_granted, "exile that card instead of putting it into your graveyard")
        .text("If a card would be put into your graveyard from anywhere this turn, exile it instead.")],
    ..oracle::FORGOTTEN_CELLAR
};

pub static CARD: CardDef = CardDef { split: Some((&WALK_IN_CLOSET, &FORGOTTEN_CELLAR)), ..oracle::WALK_IN_CLOSET_FORGOTTEN_CELLAR };

pub static EXTRA: &[&CardDef] = &[&WALK_IN_CLOSET, &FORGOTTEN_CELLAR];
