use cantrip_api::*;
use cantrip_mechanics::statics::{has_delayed_ability, your_creature_tokens};
use cantrip_mechanics::triggers::creature_card_leaves_your_graveyard;
use cantrip_mechanics::{effects, lands, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/1 green Plant creature token, then put a +1/+1 counter on each Plant you control": the token is made first, so it
/// gets a counter too (it is a Plant), as does every other Plant the controller has, all at once (CR 608.2c, 122.1).
fn plant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::PLANT, 1)?;
    effects::plus_one_counter_on_each_of_subtype_you_control::<{ lands::sub(Subtype::Plant) }>(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, creature_card_leaves_your_graveyard),
        plant,
        "Whenever one or more creature cards leave your graveyard, create a 0/1 green Plant creature token, then put a +1/+1 counter on each Plant you control.",
    )
    .once_per_batch()],
    statics: &[static_ability(Layer::Ability, your_creature_tokens, has_delayed_ability::<0>, "Creature tokens you control have \"{T}: Add one mana of any color.\"")],
    // The ability the creature tokens have.
    delayed: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    ..oracle::INSIDIOUS_ROOTS
};

pub static EXTRA: &[&CardDef] = &[&tokens::PLANT];
