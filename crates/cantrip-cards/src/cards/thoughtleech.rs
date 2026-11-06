use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever an Island an opponent controls becomes tapped" (CR 603.2).
fn when_island_opponent_controls_becomes_tapped(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Tapped { permanent } = event else { return None };
    let id = *permanent;
    let subject = Target::Object(id);
    if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Island))
        && q
            .state()
            .opponents(asker.controller)
            .collect::<Vec<PlayerId>>()
            .contains(&q.controller(id))
    {
        return Fired::about(subject);
    }
    None
}

const GAIN_LIFE: Question = Question::new("GainLife", "gain 1 life?", "", "Gain 1 life", "Don't gain life").amount(1);

/// "Whenever an Island an opponent controls becomes tapped, you may gain 1 life." (CR 608.2, 119.3).
fn gain_1_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&GAIN_LIFE))? {
        g.gain_life(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::TAPPED, when_island_opponent_controls_becomes_tapped),
            gain_1_life,
            "Whenever an Island an opponent controls becomes tapped, you may gain 1 life.",
        ),
    ],
    ..oracle::THOUGHTLEECH
};
