use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, haste_until_end_of_turn, targets};
use cantrip_oracle as oracle;

/// "Put up to two target creature cards from a single graveyard" (CR 115.1, 601.2c): the second slot's creature card lies in the
/// graveyard the first slot's does and is not that card, whichever it is (`targets::another_card_in_the_same_graveyard`); with the
/// first slot empty it may be in any graveyard, one target being a legal choice of "up to two".
fn another_creature_card_in_the_same_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::another_card_in_the_same_graveyard(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature))
}

/// "Put up to two target creature cards from a single graveyard onto the battlefield under your control. They gain haste until end
/// of turn. Sacrifice them at the beginning of the next end step." (CR 110.2a, 611.2c, 603.7c): the cards still legal targets
/// (CR 608.2b) enter together under the caster's control, the ones that did gain haste, and the delayed ability is about exactly
/// those objects, so a creature that left the battlefield before the end step is not sacrificed in its new form.
fn pull(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId)> = targets::legal_objects(g, item).into_iter().map(|card| (card, item.controller)).collect();
    let returned = g.put_all_onto_battlefield(&cards)?;
    instruction(g, |g| {
        for &id in &returned {
            haste_until_end_of_turn(g, item.object, id);
        }
    });
    if !returned.is_empty() {
        delayed::at_next_end_step_about(g, item, 0, &returned);
    }
    Ok(())
}

pub static PUSH: CardDef = CardDef { spell: spell(&[targets::TAPPED_CREATURE], effects::destroy_first_target), ..oracle::PUSH };

pub static PULL: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_CARD_IN_A_GRAVEYARD.optional(), target(another_creature_card_in_the_same_graveyard).optional()],
        pull,
    ),
    delayed: &[delayed::SACRIFICE_CREATED],
    ..oracle::PULL
};

pub static CARD: CardDef = CardDef { split: Some((&PUSH, &PULL)), ..oracle::PUSH_PULL };

pub static EXTRA: &[&CardDef] = &[&PUSH, &PULL];
