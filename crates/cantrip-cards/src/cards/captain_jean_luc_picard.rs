use cantrip_api::*;
use cantrip_mechanics::{library, select, statics, tokens};
use cantrip_oracle as oracle;

/// "Whenever you cast a creature or Spacecraft spell" (CR 601.2i, 603.2): one cast event triggers once, whether or
/// not the spell is both a creature and a Spacecraft. Spacecraft is read through this card's words (CR 612.2).
fn creature_or_spacecraft_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller
                && (q.chars(*spell).is(CardType::Creature)
                    || q.chars(*spell).has_subtype(q.words(me.source).subtype(Subtype::Spacecraft))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn make_officer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::OFFICER, 1)?;
    Ok(())
}

fn spacecraft_card(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let spacecraft = q.words(choice.source).subtype(Subtype::Spacecraft);
    Verdict::each(choice, |card| q.chars(card).has_subtype(spacecraft))
}

/// "Search your library for a Spacecraft card, put it onto the battlefield with seven charge counters on it, then
/// shuffle." Search may find no card with the stated quality (CR 701.23b); if one is found, the counters are present
/// as it enters and so see entry replacements (CR 122.6, 614.1c). The shuffle follows the search instruction (CR 608.2c).
fn fetch_spacecraft(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(select::up_to(1))
        .with(select::part(&(spacecraft_card as SelectFn), 0, ""))
        .called("Spacecraft");
    let found = library::search(
        g,
        item.controller,
        item.controller,
        item.object,
        SearchDestination::Battlefield,
        rule,
    )?;
    if let Some(&card) = found.first() { g.put_onto_battlefield_with_counters(card, item.controller, false, &[(CounterKind::Charge, 7)])?; }
    g.end_sentence();
    g.shuffle(item.controller);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_creatures_you_control,
        statics::plus_pt::<1, 1>,
        "Other creatures you control get +1/+1.",
    )],
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, creature_or_spacecraft_spell_you_cast),
            make_officer,
            "Whenever you cast a creature or Spacecraft spell, create a 1/1 red Officer creature token.",
        ),
        ability(
            tap_untapped(7, |q, asker, id| {
                q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Officer))
            }),
            fetch_spacecraft,
            "Tap seven untapped Officers you control: Search your library for a Spacecraft card, put it onto the battlefield with seven charge counters on it, then shuffle.",
        ),
    ],
    ..oracle::CAPTAIN_JEAN_LUC_PICARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
