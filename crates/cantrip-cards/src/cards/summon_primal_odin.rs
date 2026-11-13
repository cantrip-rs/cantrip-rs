use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// II: "This creature gains 'Whenever this creature deals combat damage to a player, that player loses the game.'" A continuous
/// effect with no duration that grants the triggered ability in layer 6 (CR 613.1f, 611.2a); it lasts as long as this creature
/// stays on the battlefield, and is not tied to its lore counters (the creature's text below the type line applies whatever
/// they are).
fn zantetsuken(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.obj(item.object).card;
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(me, 0)), Duration::Indefinite);
    Ok(())
}

/// III: "Draw two cards. Each player loses 2 life." (CR 121.1, 119.3): the controller draws first; then every player still in the
/// game loses 2 life, together (CR 101.4, 603.2c).
fn hall_of_sorrow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    g.end_sentence();
    let losses: Vec<(PlayerId, i32)> = g.state().apnap().map(|p| (p, 2)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

/// "That player loses the game" (CR 104.3e): the player the creature dealt combat damage to, the trigger's subject.
fn that_player_loses(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.lose_game(p);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_first_target, "I \u{2014} Gungnir \u{2014} Destroy target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, zantetsuken, "II \u{2014} Zantetsuken \u{2014} This creature gains \"Whenever this creature deals combat damage to a player, that player loses the game.\""),
        chapter(3, hall_of_sorrow, "III \u{2014} Hall of Sorrow \u{2014} Draw two cards. Each player loses 2 life."),
    ],
    delayed: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        that_player_loses,
        "Whenever this creature deals combat damage to a player, that player loses the game.",
    )],
    ..oracle::SUMMON_PRIMAL_ODIN
};
