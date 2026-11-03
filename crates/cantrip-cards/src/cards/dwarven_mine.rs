use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This land enters tapped unless you control three or more other Mountains. When this land enters untapped, create a 1/1 red Dwarf creature token." (CR 608.2, 701.26a).
fn land_enters_tapped_unless_control_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let asker = &Asker::new(entering.object, entering.controller);
    if {
        let q: &dyn Query = g;
        !((q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                (!(asker.source == id)
                    && q
                        .chars(id)
                        .has_subtype(q.words(asker.source).subtype(Subtype::Mountain)))
                    && Target::Object(asker.source)
                        .object()
                        .map(|o: ObjectId| q.controller(o)) == Some(q.controller(id))
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 3)
    } {
        entering.tapped = true;
    }
    Ok(())
}

/// "When this land enters untapped" (CR 603.2, 701.26a).
fn when_land_enters_untapped(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield)
        && (asker.source == id && !(q.state().obj(id).tapped))
    {
        return Fired::about(subject);
    }
    None
}

pub static DWARF: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(8)),
    power: Some(1),
    toughness: Some(1),
    ..token!(Dwarf)
};

/// "When this land enters untapped, create a 1/1 red Dwarf creature token." (CR 608.2, 111.1).
fn create_1_1_red_dwarf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_1 = Vec::new();
    let def_1_0 = g.card_id(&DWARF);
    tokens_1.extend(std::iter::repeat((def_1_0, item.controller)).take(1));
    let words_1 = g.words(item.object);
    g.create_tokens_worded(&tokens_1, false, None, words_1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_land_enters_untapped),
            create_1_1_red_dwarf,
            "When this land enters untapped, create a 1/1 red Dwarf creature token.",
        ),
    ],
    enters: Some(land_enters_tapped_unless_control_enters),
    ..oracle::DWARVEN_MINE
};

pub static EXTRA: &[&CardDef] = &[&DWARF];
