use cantrip_api::*;
use cantrip_mechanics::{effects, hand::exile_chosen_nonland_from_revealed_hand, targets};
use cantrip_oracle as oracle;

fn creature_or_spacecraft_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature) || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Spacecraft)))
}

fn exile_nonland(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    exile_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::return_each_target_to_hand, "Rewrite the Past — Return one or two target creature and/or Spacecraft cards from your graveyard to your hand.")
        .targets(&[target(creature_or_spacecraft_card).between(1, 2)]),
    mode(exile_nonland, "Repair the Timeline — Target opponent reveals their hand. You choose a nonland card from it. Exile that card.")
        .targets(&[targets::OPPONENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::STRAY_FROM_HISTORY };
