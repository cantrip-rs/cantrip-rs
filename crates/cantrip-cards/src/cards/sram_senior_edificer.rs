use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever you cast an Aura" (CR 603.2).
fn when_cast_aura(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player
        && (q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura))
            || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Equipment))
            || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Vehicle)))
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card." (CR 608.2, 121.1).
fn equipment_vehicle_spell_draw_card(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_cast_aura),
            equipment_vehicle_spell_draw_card,
            "Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.",
        ),
    ],
    ..oracle::SRAM_SENIOR_EDIFICER
};
