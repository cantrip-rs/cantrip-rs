use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{land_until_cast, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, target creature an opponent controls can't attack or block until your next turn" (CR
/// 508.1c, 509.1b, 611.2a): both restrictions end as the ability's controller's next turn begins.
fn restrain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let until = Duration::UntilNextTurnOf(item.controller);
        instruction(g, |g| {
            g.add_effect(item.object, creature, EffectKind::CantAttack, until);
            g.add_effect(item.object, creature, EffectKind::CantBlock, until);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, restrain, "When this creature enters, target creature an opponent controls can't attack or block until your next turn.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        land_until_cast::ability::<0>(
            cost("{2}"),
            "{2}, Exile this card from your hand: Target land gains \"{T}: Add {G}, {W}, or {U}\" until this card is cast from exile. You may cast this card for as long as it remains exiled.",
        ),
    ],
    delayed: &[mana_ability(TAP, add_one_of(&[Color::Green, Color::White, Color::Blue]), "{T}: Add {G}, {W}, or {U}.")],
    ..oracle::SPARAS_ADJUDICATORS
};
