use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell" (CR 603.2).
fn when_cast_noncreature_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player && !(q.chars(id).is(CardType::Creature)) {
        return Fired::about(subject);
    }
    None
}

/// "Whenever you cast a noncreature spell, incubate X, where X is that spell's mana value." (CR 608.2).
fn incubate_x_where_x_spells(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = {
        let q: &dyn Query = g;
        item.subject.object().map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as i16
    };
    tokens::incubate(g, item.controller, amount)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_cast_noncreature_spell),
            incubate_x_where_x_spells,
            "Whenever you cast a noncreature spell, incubate X, where X is that spell's mana value.",
        ),
    ],
    ..oracle::CHROME_HOST_SEEDSHARK
};

pub static EXTRA: &[&CardDef] = &[&tokens::INCUBATOR, &tokens::PHYREXIAN];
