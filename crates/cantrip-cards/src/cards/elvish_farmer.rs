use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a spore counter on this creature." (CR 608.2, 122.1).
fn put_spore_counter_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Spore, 1);
    Ok(())
}

/// "Remove three spore counters from this creature: Create a 1/1 green Saproling creature token." (CR 608.2).
fn create_1_1_green_saproling(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let token = g.card_id(&tokens::SAPROLING);
    g.create_tokens_worded(&[(token, item.controller)], false, None, g.words(item.object))?;
    Ok(())
}

/// "Sacrifice a Saproling: You gain 2 life." (CR 608.2, 119.3).
fn gain_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "Sacrifice a Saproling: You gain 2 life." (CR 608.2).
fn gain_2_life_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Saproling))
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
        ability(
            sacrifice(1, gain_2_life_2),
            gain_2_life,
            "Sacrifice a Saproling: You gain 2 life.",
        ),
    ],
    ..oracle::ELVISH_FARMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SAPROLING];
