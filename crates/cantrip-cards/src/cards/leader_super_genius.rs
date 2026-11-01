use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If a creature you control would connive" (CR 701.50, 614.1a): the creature, even one that has left the battlefield by the
/// time it connives (CR 701.50b), under the controller it had.
fn your_creature_would_connive(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Connive(c) if q.state().obj(c.creature).controller == asker.controller && q.chars(c.creature).is(CardType::Creature))
}

/// "... instead you draw a card, then that creature connives": the connive goes on as it was, after the card is drawn, which is what
/// the effect does in place of it (CR 614.6). Another such effect applies to it once too (CR 614.5): two draw two cards.
fn the_connive_goes_on(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

fn you_draw_a_card(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    g.draw(asker.controller, 1);
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        effects::target_connives,
        "At the beginning of combat on your turn, target creature you control connives.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    replacements: &[replacement(
        ReplacementKind::Connive,
        your_creature_would_connive,
        the_connive_goes_on,
        "you draw a card, then it connives",
    )
    .then(you_draw_a_card)],
    ..oracle::LEADER_SUPER_GENIUS
};
