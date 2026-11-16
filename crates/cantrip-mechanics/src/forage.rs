//! Forage (CR 701.61a): "to forage means 'exile three cards from your graveyard or sacrifice a Food'", as a cost.
//!
//! The two ways to forage are two ways of paying, so a spell that asks for it as an additional cost lists them beside its other
//! way: Feed the Cycle's "forage or pay {B}" is `additional_choice: &[EXILE_THREE_CARDS, SACRIFICE_A_FOOD, <pay {B}>]`
//! (CR 601.2b: the player chooses one, and only a way that can be paid is offered, CR 118.3). Exiling three cards takes
//! them from the casting player's own graveyard, never the spell itself (it is on the stack, CR 601.2a), and sacrificing a
//! Food takes any Food they control, a Food token or a Food artifact card.

use cantrip_api::*;

use crate::targets::filters;

/// "Exile three cards from your graveyard": the first way to forage (CR 701.61a).
pub const EXILE_THREE_CARDS: ExtraCost = ExtraCost { text: "Forage: exile three cards from your graveyard", cost: Cost { forage: true, ..exile_from_graveyard(3) } };

/// "Sacrifice a Food": the second way to forage (CR 701.61a).
pub const SACRIFICE_A_FOOD: ExtraCost = ExtraCost { text: "Forage: sacrifice a Food", cost: Cost { forage: true, ..Cost::sacrifice(1, filters::food) } };

const FORAGE_BY_EXILING: Question = Question::new("Forage", "forage by exiling three cards from your graveyard?", "", "Exile three cards", "Don't exile");
const FORAGE_BY_SACRIFICING: Question = Question::new("Forage", "forage by sacrificing a Food?", "", "Sacrifice a Food", "Don't sacrifice");

/// "You may forage" as the ability resolves (CR 701.61a, 608.2d, 118.12): its controller is asked whether to exile three cards from
/// their graveyard and, if they decline or cannot, whether to sacrifice a Food (a way that cannot be paid is not asked, CR 118.3).
/// Whether they foraged: the "if you do" of the sentence. Bushy Bodyguard and Curious Forager print it.
pub fn may_forage(g: &mut dyn Game, item: &StackItem) -> Result<bool, Suspend> {
    let you = item.controller;
    if g.may_pay(you, item.object, item.object, EXILE_THREE_CARDS.cost, &FORAGE_BY_EXILING)? {
        return Ok(true);
    }
    g.may_pay(you, item.object, item.object, SACRIFICE_A_FOOD.cost, &FORAGE_BY_SACRIFICING)
}

/// "Whenever you forage" (CR 701.61a): its controller paid a way to forage, as a cost or as an effect asked (`Event::Foraged`).
/// Each forage triggers it once, so foraging twice triggers it twice.
pub fn you_foraged(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Foraged { player, .. } = *event else { return None };
    (player == me.controller).then_some(Fired { subject: Target::Player(player), data: [0; 2] })
}

/// "Whenever you forage".
pub const YOU_FORAGE: Trigger = when_asked(EventMask::FORAGED, you_foraged);
