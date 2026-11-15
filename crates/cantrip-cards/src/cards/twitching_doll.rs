use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "{T}: Add one mana of any color. Put a nest counter on this creature." (CR 605.3a, 106.3, 122.1): the mana is added first, then the
/// counter goes on, the way the text reads.
fn add_one_mana_of_any_color_and_a_nest_counter(g: &mut dyn Game, asker: &Asker, chosen: Option<Color>) -> Result<(), Suspend> {
    let Some(color) = chosen else { return Ok(()) };
    g.add_mana(asker.controller, color.mana(), 1);
    g.add_counters(asker.source, CounterKind::Nest, 1);
    Ok(())
}

/// "Create a 2/2 green Spider creature token with reach for each counter on this creature" (CR 111.1, 122.1, 608.2h): counters of every
/// kind, read from the Doll as it last existed, since sacrificing it is a cost (CR 602.2b).
fn create_a_spider_for_each_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = effects::counters_on(g, item.object);
    tokens::create(g, item.controller, &tokens::SPIDER_2_2, counters)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add_by(add_one_mana_of_any_color_and_a_nest_counter).choose_color().colors(ColorSet(31)).yields_any(1),
            "{T}: Add one mana of any color. Put a nest counter on this creature.",
        ),
        ability(
            TAP.and(SACRIFICE_THIS),
            create_a_spider_for_each_counter,
            "{T}, Sacrifice this creature: Create a 2/2 green Spider creature token with reach for each counter on this creature. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::TWITCHING_DOLL
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIDER_2_2];
