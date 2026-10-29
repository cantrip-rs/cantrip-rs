use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each creature you control. Draw a card."
fn counters_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_each_creature_you_control(g, item)?;
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{6}").and(DISCARD_THIS), counters_and_draw, "Channel \u{2014} {6}, Discard this card: Put a +1/+1 counter on each creature you control. Draw a card.")
            .from(ZoneKind::Hand),
    ],
    ..oracle::ACTION_NEWS_CREW
};
