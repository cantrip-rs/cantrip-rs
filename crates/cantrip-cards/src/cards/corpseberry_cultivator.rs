use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::forage;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_oracle as oracle;

/// "You may forage." (CR 701.61a): asked as the trigger resolves, by exiling three cards from its controller's graveyard or
/// sacrificing a Food; foraging this way is what the second ability sees.
fn you_may_forage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    forage::may_forage(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            you_may_forage,
            "At the beginning of combat on your turn, you may forage. (Exile three cards from your graveyard or sacrifice a Food.)",
        ),
        triggered(forage::YOU_FORAGE, effects::plus_one_counter_on_this, "Whenever you forage, put a +1/+1 counter on this creature."),
    ],
    ..oracle::CORPSEBERRY_CULTIVATOR
};
