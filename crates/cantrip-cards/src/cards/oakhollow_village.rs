use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel you control that entered the battlefield this turn" (CR 122.1,
/// 608.2c): every permanent its controller controls as it resolves that has one of the subtypes (read through the words of this
/// card's text, CR 612) and entered this turn (CR 400.7), all of them at once (CR 101.4).
fn counter_on_each_new_frog_rabbit_raccoon_or_squirrel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let subtypes = [words.subtype(Subtype::Frog), words.subtype(Subtype::Rabbit), words.subtype(Subtype::Raccoon), words.subtype(Subtype::Squirrel)];
    let objects: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.history().entered_this_turn(id) && subtypes.iter().any(|&s| g.chars(id).has_subtype(s)))
        .collect();
    effects::counters_on_each(g, &objects, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP, add(Mana::G).only_for(ManaRestriction::CREATURE_SPELLS), "{T}: Add {G}. Spend this mana only to cast a creature spell."),
        ability(
            cost("{G}, {T}"),
            counter_on_each_new_frog_rabbit_raccoon_or_squirrel,
            "{G}, {T}: Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel you control that entered the battlefield this turn.",
        ),
    ],
    ..oracle::OAKHOLLOW_VILLAGE
};
