use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step." (CR 608.2, 611.2a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let candidates_3 = {
        let q: &dyn Query = g;
        q.state().opponents(item.controller).collect::<Vec<PlayerId>>()
    };
    let Some(chosen_3) = choose::player(
        g,
        item.controller,
        item.object,
        candidates_3,
        "gains control of this land",
        0,
    )? else { return Ok(()) };
    g.gain_control(item.object, item.object, chosen_3, Duration::Indefinite);
    Ok(())
}

/// "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step." (CR 605.3a, 106.3).
fn add_one_mana_any_color_mana(
    g: &mut dyn Game,
    asker: &Asker,
    chosen: Option<Color>,
) -> Result<(), Suspend> {
    let Some(color) = chosen else { return Ok(()) };
    g.add_mana(asker.controller, color.mana(), 1);
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: None,
        },
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
            TAP,
            add_by(add_one_mana_any_color_mana)
                .choose_color()
                .colors(ColorSet(31))
                .yields_any(1),
            "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_2,
            "An opponent gains control of this land at the beginning of the next end step.",
        ),
    ],
    ..oracle::RAINBOW_VALE
};
