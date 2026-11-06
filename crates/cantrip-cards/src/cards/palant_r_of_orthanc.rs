use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_mechanics::{scry, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("HaveControllerDraw", "have its controller draw a card?", "If you don't, its controller mills a card for each influence counter on it and you lose life equal to the total mana value of those cards", "Let them draw", "Mill, lose life");


/// Put an influence counter on this and scry 2. Then target opponent may have you draw a card; if they don't, you mill
/// X cards, X being the influence counters on this (as it last was if it has gone, CR 608.2h), and they lose life equal
/// to the total mana value of those cards (wherever they went, CR 701.17c).
fn offer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(them) = item.target(0) else { return Ok(()) };
    let you = item.controller;
    let this = item.object;
    if targets::permanent(g, this) {
        g.add_counters(this, CounterKind::Influence, 1);
    }
    scry(g, you, this, 2)?;
    let influence = g.obj(this).counters(CounterKind::Influence).max(0) as u32;
    if effects::ask_may(g, them, this, Purpose::Ask(&ASK))? {
        g.draw(you, 1);
        return Ok(());
    }
    if influence == 0 {
        return Ok(());
    }
    let milled = g.mill(you, influence);
    let total: u32 = milled.iter().map(|&card| g.mana_value(card)).sum();
    if total > 0 {
        g.lose_life(them,(total as i32) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_END_STEP, offer, "At the beginning of your end step, put an influence counter on Palant\u{ed}r of Orthanc and scry 2. Then target opponent may have you draw a card. If that player doesn't, you mill X cards, where X is the number of influence counters on Palant\u{ed}r of Orthanc, and that player loses life equal to the total mana value of those cards.")
        .targets(&[targets::OPPONENT])],
    ..oracle::PALANT_R_OF_ORTHANC
};
