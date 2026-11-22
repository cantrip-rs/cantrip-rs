use cantrip_api::*;
use cantrip_mechanics::{hand, permanents_with_subtype_you_control};
use cantrip_oracle as oracle;

/// "You may put a land card from your hand onto the battlefield." (CR 608.2d, 305.4): any land card of the hand may be chosen, or
/// none; nothing is asked with no land card there. Putting a land onto the battlefield is not playing it, so it uses no land play
/// and works after the turn's land drop (the ruling).
fn may_put_a_land_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_onto_battlefield(g, item.controller, item.object, |chars| chars.is(CardType::Land))?;
    Ok(())
}

/// "Until end of turn, this creature's base power becomes equal to the number of Towns you control." (CR 613.4b, 611.2a): the number
/// is counted as the ability resolves and stays fixed for the turn (the ruling); its toughness is left alone. "Town" is a word of
/// this card's text (CR 612).
fn base_power_becomes_the_number_of_towns(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let town = g.words(item.object).subtype(Subtype::Town);
    let towns = permanents_with_subtype_you_control(g, item.controller, town) as i32;
    g.add_effect(item.object, item.object, EffectKind::SetPower(towns), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, may_put_a_land_from_hand, "{T}: You may put a land card from your hand onto the battlefield."),
        ability(
            Cost::mana(mana("{3}")),
            base_power_becomes_the_number_of_towns,
            "{3}: Until end of turn, this creature's base power becomes equal to the number of Towns you control.",
        ),
    ],
    ..oracle::PUPU_UFO
};
