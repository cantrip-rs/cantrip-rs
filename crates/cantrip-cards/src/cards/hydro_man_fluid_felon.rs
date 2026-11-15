use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{self, YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "If Hydro-Man is a creature" (CR 603.4): an intervening "if", checked as the ability triggers and as it resolves.
fn is_a_creature(q: &dyn Query, asker: &Asker) -> bool {
    is_live(q, asker.source) && q.is_on_battlefield(asker.source) && q.chars(asker.source).is(CardType::Creature)
}

/// "Untap Hydro-Man. Until your next turn, he becomes a land and gains '{T}: Add {U}.'" The land is exactly a land
/// (CR 205.1a, 613.1d): it is no longer a creature and loses its creature types, and keeps Legendary. The effect ends as
/// its controller's next turn begins (CR 611.2a), so he is never a land and a creature at once.
fn become_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    if !is_live(g, me) || !g.is_on_battlefield(me) {
        return Ok(());
    }
    g.untap(me);
    g.end_sentence();
    let until = Duration::UntilNextTurnOf(item.controller);
    instruction(g, |g| {
        g.add_effect(me, me, EffectKind::SetTypes(TypeSet::of(&[CardType::Land])), until);
        g.add_effect(me, me, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), until);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_of_color::<{ Color::Blue as u8 }>), effects::this_gets::<1, 1>, "Whenever you cast a blue spell, if Hydro-Man is a creature, he gets +1/+1 until end of turn.")
            .condition(is_a_creature),
        triggered(YOUR_END_STEP, become_a_land, "At the beginning of your end step, untap Hydro-Man. Until your next turn, he becomes a land and gains \"{T}: Add {U}.\" (He's not a creature during that time.)"),
    ],
    delayed: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")],
    ..oracle::HYDRO_MAN_FLUID_FELON
};
