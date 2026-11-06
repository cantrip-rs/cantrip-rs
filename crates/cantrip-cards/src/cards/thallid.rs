use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a spore counter on this creature." (CR 608.2, 122.1).
fn put_spore_counter_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Spore, 1));
        }
    }
    g.add_counters_together(&counters_0);
    Ok(())
}

/// "Remove three spore counters from this creature: Create a 1/1 green Saproling creature token." (CR 608.2).
fn create_1_1_green_saproling(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut tokens_1 = Vec::new();
    let def_1_0 = g.card_id(&tokens::SAPROLING);
    tokens_1.extend(std::iter::repeat((def_1_0, item.controller)).take(1));
    let words_1 = g.words(item.object);
    g.create_tokens_worded(&tokens_1, false, None, words_1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_spore_counter_creature,
            "At the beginning of your upkeep, put a spore counter on this creature.",
        ),
        ability(
            remove_counters(CounterKind::Spore, 3),
            create_1_1_green_saproling,
            "Remove three spore counters from this creature: Create a 1/1 green Saproling creature token.",
        ),
    ],
    ..oracle::THALLID
};

pub static EXTRA: &[&CardDef] = &[&tokens::SAPROLING];
