use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Creatures you control get +X/+X until end of turn, where X is the number of Foods you control" (CR 611.2, 611.2c, 608.2h): X is
/// counted once as the ability resolves, the enchantment already sacrificed as a cost (it is no Food), and the creatures are those
/// the controller controls then.
fn creatures_you_control_get_plus_x_where_x_is_the_number_of_foods_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let foods = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(Subtype::Food)).count() as i32;
    effects::pump_creatures_you_control(g, item, foods, foods);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_a_food_token,
            "When this enchantment enters, create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")",
        ),
        ability(
            cost("{5}{G}{G}").and(SACRIFICE_THIS),
            creatures_you_control_get_plus_x_where_x_is_the_number_of_foods_you_control,
            "{5}{G}{G}, Sacrifice this enchantment: Creatures you control get +X/+X until end of turn, where X is the number of Foods you control. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    // "{T}: Add {G}.", the mana ability the Foods are given: the Food is its source (CR 113.6).
    delayed: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    // "Foods you control have ..." (CR 613.1f, layer 6): the Foods are artifacts with the Food subtype, tokens or not, each one's own
    // Food ability kept (CR 111.10b).
    statics: &[static_ability(
        Layer::Ability,
        statics::your_of_subtype::<{ lands::sub(Subtype::Food) }>,
        statics::grant_delayed_ability,
        "Foods you control have \"{T}: Add {G}.\"",
    )],
    ..oracle::NIGHT_OF_THE_SWEETS_REVENGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
