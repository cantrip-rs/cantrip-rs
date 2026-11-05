use cantrip_api::*;
use cantrip_oracle as oracle;

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

/// III: a +1/+1 counter on each creature you control, which gain vigilance, trample and indestructible until end of
/// turn.
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = creatures_you_control(g, item.controller);
    let puts: Vec<(ObjectId, CounterKind, i64)> = creatures.iter().map(|&id| (id, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    g.end_sentence();
    let keywords = Keywords::of(&[Keyword::Vigilance, Keyword::Trample, Keyword::Indestructible]);
    instruction(g, |g| {
        for id in creatures {
            g.add_effect(item.object, id, EffectKind::AddKeywords(keywords), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, grant_mana, "I \u{2014} Until your next turn, creatures you control gain \"{T}: Add one mana of any color.\""),
        chapter(2, grant_mana, "II \u{2014} Until your next turn, creatures you control gain \"{T}: Add one mana of any color.\""),
        chapter(3, rally, "III \u{2014} Put a +1/+1 counter on each creature you control. Those creatures gain vigilance, trample, and indestructible until end of turn."),
    ],
    // The ability chapters I and II grant.
    delayed: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    ..oracle::SONG_OF_FREYALISE
};
