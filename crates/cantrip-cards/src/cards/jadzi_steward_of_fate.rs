use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, prepare, tokens, triggers};
use cantrip_oracle as oracle;

/// "Draw two cards, then discard two cards" (CR 121.1, 701.9): the two are chosen from the hand once the two are in it.
fn draw_two_discard_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    discard_chosen(g, item.controller, item.object, 2)?;
    Ok(())
}

/// "Create X 0/0 green and blue Fractal creature tokens, then put X +1/+1 counters on each Fractal you control." The tokens are
/// made together (CR 101.4); the sentence ends at "then" (CR 608.2c), so the counters go on every Fractal the controller has
/// by then, the new ones and any older, read through the words of this spell's text (CR 612). With X of 0 nothing is made
/// and no counters are put.
fn oracles_gift(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    tokens::create(g, you, &tokens::FRACTAL, item.x as u32)?;
    g.end_sentence();
    let fractal = g.words(item.object).subtype(Subtype::Fractal);
    let fractals: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).has_subtype(fractal)).collect();
    let puts: Vec<(ObjectId, CounterKind, i16)> = fractals.into_iter().map(|id| (id, CounterKind::PlusOne, item.x as i16)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

/// Jadzi's prepare spell (CR 722): not a card, only ever a copy, cast from exile (CR 722.3c).
pub static ORACLES_GIFT: CardDef = CardDef {
    name: "Oracle's Gift",
    cost: mana("{X}{X}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], oracles_gift),
    text: "Create X 0/0 green and blue Fractal creature tokens, then put X +1/+1 counters on each Fractal you control.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, draw_two_discard_two, "When Jadzi enters, draw two cards, then discard two cards.")],
    enters: Some(prepare::enters_prepared),
    prepare: Some(&ORACLES_GIFT),
    ..oracle::JADZI_STEWARD_OF_FATE
};

pub static EXTRA: &[&CardDef] = &[&ORACLES_GIFT, &tokens::FRACTAL];
