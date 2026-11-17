use cantrip_api::*;
use cantrip_mechanics::select::{self, up_to, Selection};
use cantrip_mechanics::{effects, hand, targets};
use cantrip_oracle as oracle;

/// I — "The owner of up to one target creature draws two cards, then shuffles that creature into their library" (CR 701.24a,
/// 608.2c): the owner draws, whoever controls it, and only then is the creature shuffled in; no target, nothing at all (CR 115.1d).
fn owner_draws_two_then_shuffles(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let owner = g.obj(creature).owner;
    g.draw(owner, 2);
    g.end_sentence();
    g.put_into_library(&[creature], LibraryPlace::ShuffledIn);
    Ok(())
}

/// II — "You may put a Spacecraft card from your hand or graveyard onto the battlefield" (CR 608.2d): the two zones are shown
/// together, up to one is taken, and nothing is asked when neither holds one.
fn may_put_a_spacecraft_from_hand_or_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_from_hand_or_graveyard_onto_battlefield(g, item.controller, item.object, A_SPACECRAFT)?;
    Ok(())
}

/// "A Spacecraft card": the subtype read through this card's words (CR 612.2).
const SPACECRAFT_CARD: Part = select::subtype_card(Subtype::Spacecraft);

/// "Up to one" of them, so none may be taken (CR 608.2d).
const A_SPACECRAFT: Selection = Selection::of(up_to(1), &[SPACECRAFT_CARD]).called("Spacecraft");

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, owner_draws_two_then_shuffles, "I — The owner of up to one target creature draws two cards, then shuffles that creature into their library.")
            .targets(&[targets::CREATURE.optional()]),
        chapter(2, may_put_a_spacecraft_from_hand_or_graveyard, "II — You may put a Spacecraft card from your hand or graveyard onto the battlefield."),
        // III — "You and target opponent each draw two cards" (CR 121.2c): in turn order from the active player, and with the
        // opponent no longer a legal target nothing is drawn at all (CR 608.2b).
        chapter(3, |g, item| { effects::you_and_target_player_each_draw(g, item, 0, 2); Ok(()) }, "III — You and target opponent each draw two cards.")
            .targets(&[targets::OPPONENT]),
    ],
    ..oracle::UNIFICATION
};
