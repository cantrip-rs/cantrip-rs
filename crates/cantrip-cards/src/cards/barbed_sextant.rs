use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a card at the beginning of the next turn's upkeep." (CR 605.3a, 106.3).
fn add_one_mana_any_color_mana(
    g: &mut dyn Game,
    asker: &Asker,
    chosen: Option<Color>,
) -> Result<(), Suspend> {
    let Some(color) = chosen else { return Ok(()) };
    g.add_mana(asker.controller, color.mana(), 1);
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
        asker.source,
        asker.controller,
        AbilityRef::delayed(g.card_id(&CARD), 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            cost("{1}").and(SACRIFICE_THIS).and(TAP),
            add_by(add_one_mana_any_color_mana)
                .choose_color()
                .colors(ColorSet(31))
                .yields_any(1),
            "{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            |g, item| effects::draw(g, item, 1),
            "Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    ..oracle::BARBED_SEXTANT
};
