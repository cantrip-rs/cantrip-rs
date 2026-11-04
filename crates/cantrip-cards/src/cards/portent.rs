use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, top_of_library};
use cantrip_oracle as oracle;

const SHUFFLE: Question = Question::new("ShuffleTargetLibrary", "have the targeted player shuffle their library?", "Chosen after putting the cards back", "Shuffle", "Don't shuffle");

/// Look at and reorder the target player's top three cards, then optionally shuffle (CR 608.2c, 701.24a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let cards = top_of_library(g, player, 3);
    g.look_at(item.controller, &cards);
    let order = library::ask_library_order(g, item.controller, item.object, &cards, DestinationKind::LibraryTop)?;
    g.arrange_library(player, &order, &[]);
    g.end_sentence();
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&SHUFFLE))? {
        g.shuffle(player);
    }
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
        item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::PORTENT
};
