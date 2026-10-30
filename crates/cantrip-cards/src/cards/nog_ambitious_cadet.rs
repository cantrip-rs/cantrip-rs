use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

fn become_scout(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rogue = g.words(item.object).subtype(Subtype::Rogue);
    if !g.chars(item.object).has_subtype(rogue) {
        return Ok(());
    }

    let ferengi = g.words(item.object).subtype(Subtype::Ferengi);
    let scout = g.words(item.object).subtype(Subtype::Scout);
    let card = g.obj(item.object).card;
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetCreatureType(scout), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(ferengi), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::SetPt(3, 4), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(card, 0)), Duration::Indefinite);
    });
    Ok(())
}

fn become_officer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let scout = g.words(item.object).subtype(Subtype::Scout);
    if !g.chars(item.object).has_subtype(scout) {
        return Ok(());
    }

    let ferengi = g.words(item.object).subtype(Subtype::Ferengi);
    let officer = g.words(item.object).subtype(Subtype::Officer);
    let card = g.obj(item.object).card;
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetCreatureType(officer), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(ferengi), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(card, 1)), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(card, 2)), Duration::Indefinite);
    });
    Ok(())
}

fn nog_becomes_tapped(_q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent } if *permanent == asker.source => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), become_scout, "{1}: If Nog is a Rogue, he becomes a Ferengi Scout with base power and toughness 3/4 and gains \"Whenever Nog becomes tapped, surveil 2.\""),
        ability(cost("{3}"), become_officer, "{3}: If Nog is a Scout, he becomes a Ferengi Officer and gains \"You may play lands from your graveyard\" and \"You may play an additional land on each of your turns.\""),
    ],
    delayed: &[
        triggered(when_asked(EventMask::TAPPED, nog_becomes_tapped), |g, item| effects::surveil(g, item, 2), "Whenever Nog becomes tapped, surveil 2."),
        granted_play_rule(PlayRule::LandsFromGraveyard, "You may play lands from your graveyard."),
        granted_play_rule(PlayRule::ExtraLandPlays(1), "You may play an additional land on each of your turns."),
    ],
    ..oracle::NOG_AMBITIOUS_CADET
};
