use cantrip_api::*;
use cantrip_mechanics::{effects, subtype_cards_in_graveyard, triggers};
use cantrip_oracle as oracle;

/// "If there is an Elf card in your graveyard and this creature has a -1/-1 counter on it" (CR 603.4): checked as the end step
/// begins and again as the ability resolves.
fn an_elf_in_your_graveyard_and_a_counter_on_this(q: &dyn Query, asker: &Asker) -> bool {
    let elf = q.words(asker.source).subtype(Subtype::Elf);
    subtype_cards_in_graveyard(q, asker.controller, elf) > 0 && q.obj(asker.source).counters(CounterKind::MinusOne) > 0
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<3>),
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        effects::remove_a_minus_one_counter_from_this,
        "At the beginning of your end step, if there is an Elf card in your graveyard and this creature has a -1/-1 counter on it, remove a -1/-1 counter from this creature.",
    )
    .condition(an_elf_in_your_graveyard_and_a_counter_on_this)],
    ..oracle::CREAKWOOD_SAFEWRIGHT
};
