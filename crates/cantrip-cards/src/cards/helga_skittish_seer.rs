use cantrip_api::*;
use cantrip_mechanics::{effects, mana};
use cantrip_oracle as oracle;

/// "Whenever you cast a creature spell with mana value 4 or greater" (CR 603.2, 601.2i, 202.3): a creature spell its controller
/// casts, with the mana value it has on the stack - the X chosen for it counts (CR 202.3e).
fn you_cast_a_creature_spell_with_mana_value_4_or_greater(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.chars(*spell).is(CardType::Creature) && q.mana_value(*spell) >= 4 => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "You draw a card, gain 1 life, and put a +1/+1 counter on Helga" (CR 121.1, 119.1, 122.1, 608.2c): in that order; the counter
/// goes on only while Helga is still the permanent that triggered (CR 400.7).
fn draw_gain_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw_a_card_and_gain_life::<1>(g, item)?;
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_creature_spell_with_mana_value_4_or_greater),
            draw_gain_and_grow,
            "Whenever you cast a creature spell with mana value 4 or greater, you draw a card, gain 1 life, and put a +1/+1 counter on Helga.",
        ),
        mana_ability(
            TAP,
            mana::current_power_of_one_color().only_for(ManaRestriction::creature_mana_value_at_least_or_x(4)),
            "{T}: Add X mana of any one color, where X is Helga's power. Spend this mana only to cast creature spells with mana value 4 or greater or creature spells with {X} in their mana costs.",
        ),
    ],
    ..oracle::HELGA_SKITTISH_SEER
};
