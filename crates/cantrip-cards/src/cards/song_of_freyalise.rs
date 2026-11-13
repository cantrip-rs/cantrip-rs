use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Chapter III's keywords: vigilance, trample and indestructible.
const RALLY: Keywords = Keywords::of(&[Keyword::Vigilance, Keyword::Trample, Keyword::Indestructible]);

fn creatures_you_control(g: &dyn Game, player: PlayerId) -> Vec<ObjectId> {
    g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect()
}

/// I, II: "Until your next turn, creatures you control gain '{T}: Add one mana of any color.'" The creatures are the
/// ones their controller has as the chapter resolves, fixed then (CR 611.2c); the grant outlives the Saga.
fn grant_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.obj(item.object).card;
    let creatures = creatures_you_control(g, item.controller);
    instruction(g, |g| {
        for id in creatures {
            g.add_effect(item.object, id, EffectKind::AddAbility(AbilityRef::delayed(me, 0)), Duration::UntilNextTurnOf(item.controller));
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, grant_mana, "I \u{2014} Until your next turn, creatures you control gain \"{T}: Add one mana of any color.\""),
        chapter(2, grant_mana, "II \u{2014} Until your next turn, creatures you control gain \"{T}: Add one mana of any color.\""),
        chapter(3, effects::plus_one_counter_on_each_creature_you_control_which_gain_until_end_of_turn::<{ RALLY.0 }>,"III \u{2014} Put a +1/+1 counter on each creature you control. Those creatures gain vigilance, trample, and indestructible until end of turn."),
    ],
    // The ability chapters I and II grant.
    delayed: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    ..oracle::SONG_OF_FREYALISE
};
