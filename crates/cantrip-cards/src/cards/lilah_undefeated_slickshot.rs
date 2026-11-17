use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever you cast a multicolored instant or sorcery spell from your hand" (CR 105.2b, 601.2a, 603.2): the spell is judged as it
/// is on the stack, two or more colors, and about it as it waits for the trigger to resolve.
fn you_cast_a_multicolored_instant_or_sorcery_from_your_hand(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::you_cast_an_instant_or_sorcery_from_your_hand(q, me, event).filter(|_| matches!(event, Event::SpellCast { spell, .. } if q.chars(*spell).colors.0.count_ones() >= 2))
}

/// "Exile that spell instead of putting it into your graveyard as it resolves. If you do, it becomes plotted." (CR 614.1a, 702.170a):
/// a replacement of the spell's move to the graveyard as it resolves, after its effects have happened; a spell that is countered,
/// or that never resolves, goes to the graveyard, and one whose own effects exile it is not exiled again. Once exiled it can be
/// cast on a later turn, as a sorcery, without paying its mana cost.
fn exile_it_and_plot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        g.exile_spell_instead(spell, ExileInstead::Plot);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_multicolored_instant_or_sorcery_from_your_hand),
            exile_it_and_plot,
            "Whenever you cast a multicolored instant or sorcery spell from your hand, exile that spell instead of putting it into your graveyard as it resolves. If you do, it becomes plotted.",
        ),
    ],
    ..oracle::LILAH_UNDEFEATED_SLICKSHOT
};
