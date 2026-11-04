use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// III, "Draw a card for each tapped creature your opponents control" (CR 121.1, 608.2h): counted as the chapter resolves,
/// the creatures as the layers make them (CR 613), all of them the controller's opponents' and tapped, and drawn at once.
fn diamond_dust(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let tapped = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| opponents.contains(&g.controller(id)) && g.obj(id).tapped && g.chars(id).is(CardType::Creature))
        .count() as u32;
    effects::draw(g, item, tapped)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::tap_and_stun::<1>, "I \u{2014} Heavenly Strike \u{2014} Tap target creature an opponent controls. Put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, effects::tap_and_stun::<1>, "II \u{2014} Heavenly Strike \u{2014} Tap target creature an opponent controls. Put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(3, diamond_dust, "III \u{2014} Diamond Dust \u{2014} Draw a card for each tapped creature your opponents control."),
    ],
    ..oracle::SUMMON_SHIVA
};
