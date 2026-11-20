use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Add one mana of any color, then put an unlock counter on this artifact (CR 605.3a, 122.6): both as the mana ability resolves,
/// in the order printed. The counter goes on the artifact only while it is still on the battlefield.
fn add_one_of_any_color_and_put_an_unlock_counter(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), 1);
    g.add_counters(asker.source, CounterKind::Unlock, 1);
    Ok(())
}

/// "Surveil 3, then draw three cards" (CR 701.25, 121.1, 608.2c): the surveil is complete before the draw.
fn surveil_3_then_draw_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 3)?;
    effects::draw(g, item, 3)
}

/// "Activate only if this artifact has five or more unlock counters on it" (CR 602.5): asked as the ability is activated.
fn has_five_unlock_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Unlock) >= 5
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP.and(Cost { collect_evidence: 3, ..Cost::NONE }),
            add_any_color(1).by(add_one_of_any_color_and_put_an_unlock_counter),
            "{T}, Collect evidence 3: Add one mana of any color. Put an unlock counter on this artifact. (To collect evidence 3, exile cards with total mana value 3 or greater from your graveyard.)",
        ),
        ability(
            SACRIFICE_THIS,
            surveil_3_then_draw_3,
            "Sacrifice this artifact: Surveil 3, then draw three cards. Activate only if this artifact has five or more unlock counters on it.",
        )
        .condition(has_five_unlock_counters),
    ],
    ..oracle::CRYPTEX
};
