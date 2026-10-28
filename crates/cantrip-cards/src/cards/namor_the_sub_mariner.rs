use cantrip_api::*;
use cantrip_mechanics::devotion::symbols_of;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// The 1/1 blue Merfolk creature token.
pub static MERFOLK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Merfolk)
};

/// "Namor's power is equal to the number of Merfolk you control" (CR 604.3, layer 7a): a characteristic-defining ability,
/// Namor himself among them, the type read through the words of his text (CR 612.2).
fn power_is_merfolk_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let merfolk = q.words(source).subtype(Subtype::Merfolk);
    chars.power = Some(lands::count_you_control(q, source, &|c| c.has_subtype(merfolk)));
}

/// "Whenever you cast a noncreature spell with one or more blue mana symbols in its mana cost" (CR 601.2i, 202.2): the spell
/// as it is cast, its blue mana symbols (hybrid and Phyrexian ones among them) counted in its mana cost; the number is what
/// "that many" means (`Fired::data`).
fn noncreature_spell_with_blue_symbols(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && !q.chars(*spell).is(CardType::Creature) => {
            let blue = symbols_of(q, *spell, Color::Blue);
            (blue > 0).then_some(Fired { subject: Target::None, data: [blue, 0] })
        }
        _ => None,
    }
}

/// "Create that many 1/1 blue Merfolk creature tokens", together.
fn that_many_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MERFOLK, item.data[0])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, power_is_merfolk_you_control, "Namor's power is equal to the number of Merfolk you control.")
        .lost_pt(0, 4)],
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, noncreature_spell_with_blue_symbols),
        that_many_merfolk,
        "Whenever you cast a noncreature spell with one or more blue mana symbols in its mana cost, create that many 1/1 blue Merfolk creature tokens.",
    )],
    ..oracle::NAMOR_THE_SUB_MARINER
};

pub static EXTRA: &[&CardDef] = &[&MERFOLK];
