use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// Add white and schedule the sacrifice after the fourth and subsequent activations this turn (CR 605.3a, 603.7).
fn add_white(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::W, 1);
    let card = g.card_id(&CARD);
    let activations = history::this_ability_activations_this_turn(g, asker);
    if activations >= 4 {
        g.delayed_trigger(
            DelayedWhen::Step { step: Step::End, on_turn_of: None },
            asker.source,
            asker.controller,
            AbilityRef::delayed(card, 0),
            Target::None,
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        cost("{1}"),
        Adds { pool: ManaPool([1, 0, 0, 0, 0, 0]), ..add_by(add_white) },
        "{1}: Add {W}. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.",
    )],
    delayed: &[delayed_trigger(
        effects::sacrifice_self,
        "If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.",
    )],
    ..oracle::FARRELITE_PRIEST
};
