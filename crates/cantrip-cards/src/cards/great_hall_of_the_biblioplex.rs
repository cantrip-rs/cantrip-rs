use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, effects};
use cantrip_oracle as oracle;

/// "{5}: If this land isn't a creature, it becomes a 2/4 Wizard creature with '...' It's still a land." The check is
/// made as the ability resolves; the change has no duration (CR 611.2a), so it lasts while the land stays where it is.
/// The text names no colors, so they are not changed.
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    if g.chars(me).is(CardType::Creature) {
        return Ok(());
    }
    let card = g.obj(me).card;
    let animation = Animation {
        power: 2,
        toughness: 4,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(me).subtype(Subtype::Wizard)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: Some(AbilityRef::delayed(card, 0)),
    };
    animate(g, me, me, &animation, Duration::Indefinite);
    Ok(())
}

/// "Whenever you cast an instant or sorcery spell" (CR 601.2i): cast, not copied.
fn instant_or_sorcery_you_cast(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == q.controller(source) && (q.chars(*spell).is(CardType::Instant) || q.chars(*spell).is(CardType::Sorcery)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP.and(life(1)), add_any_color(1).only_for(ManaRestriction::INSTANT_OR_SORCERY_SPELLS), "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."),
        ability(cost("{5}"), becomes_creature, "{5}: If this land isn't a creature, it becomes a 2/4 Wizard creature with \"Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn.\" It's still a land."),
    ],
    delayed: &[triggered(when(EventMask::SPELL_CAST, instant_or_sorcery_you_cast), effects::this_gets::<1, 0>, "Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn.")],
    ..oracle::GREAT_HALL_OF_THE_BIBLIOPLEX
};
