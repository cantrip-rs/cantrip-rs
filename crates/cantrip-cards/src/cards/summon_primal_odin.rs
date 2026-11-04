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

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_first_target, "I \u{2014} Gungnir \u{2014} Destroy target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, zantetsuken, "II \u{2014} Zantetsuken \u{2014} This creature gains \"Whenever this creature deals combat damage to a player, that player loses the game.\""),
        chapter(3, |g, item| effects::draw_then_each_player_loses_life(g, item, 2, 2), "III \u{2014} Hall of Sorrow \u{2014} Draw two cards. Each player loses 2 life."),
    ],
    delayed: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        effects::that_player_loses_the_game,
        "Whenever this creature deals combat damage to a player, that player loses the game.",
    )],
    ..oracle::SUMMON_PRIMAL_ODIN
};
