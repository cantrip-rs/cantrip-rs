use cantrip_api::*;
use cantrip_mechanics::{lands, mana, targets, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, this land becomes a 3/3 black Beholder creature with menace and 'Whenever this creature attacks, exile
/// target card from defending player's graveyard.' It's still a land." (CR 613.1d, 613.1f, 613.4b): the color and creature
/// type are the ones its own text has now (CR 612). Each activation grants its own instance of the trigger (the ruling).
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ability = Some(AbilityRef::delayed(item.ability.card, 0));
    lands::becomes_creature_with_ability_until_end_of_turn(g, item, (3, 3), ColorSet::of(Color::Black), Subtype::Beholder, Keywords::of(&[Keyword::Menace]), ability);
    Ok(())
}

/// "Exile target card from defending player's graveyard."
fn exile_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_all(&[card]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_if_two_or_more_other_lands),
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(cost("{3}{B}"), becomes_creature, "{3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature with menace and \"Whenever this creature attacks, exile target card from defending player's graveyard.\" It's still a land."),
    ],
    delayed: &[triggered(triggers::THIS_ATTACKS, exile_card, "Whenever this creature attacks, exile target card from defending player's graveyard.")
        .targets(&[targets::CARD_IN_DEFENDING_PLAYERS_GRAVEYARD])],
    ..oracle::HIVE_OF_THE_EYE_TYRANT
};
