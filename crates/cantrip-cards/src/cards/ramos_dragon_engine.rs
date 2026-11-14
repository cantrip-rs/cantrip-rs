use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on [this creature] for each of that spell's colors" (CR 122.1, 105.2): the spell's colors as this
/// resolves, or as it last was if it has left the stack (CR 608.2h), so a spell countered in the meantime still counts.
/// Nothing for a colorless spell, and nothing once the Dragon has left the battlefield (CR 400.7).
fn plus_one_counter_for_each_of_that_spells_colors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = triggers::colors_of_that_spell(g, item);
    g.add_counters(item.object, CounterKind::PlusOne, colors as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_about_it),
            plus_one_counter_for_each_of_that_spells_colors,
            "Whenever you cast a spell, put a +1/+1 counter on Ramos for each of that spell's colors.",
        ),
        mana_ability(
            remove_counters(CounterKind::PlusOne, 5),
            add_each(&[Mana::W, Mana::W, Mana::U, Mana::U, Mana::B, Mana::B, Mana::R, Mana::R, Mana::G, Mana::G]),
            "Remove five +1/+1 counters from Ramos: Add {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}. Activate only once each turn.",
        )
        .once_each_turn(),
    ],
    ..oracle::RAMOS_DRAGON_ENGINE
};
