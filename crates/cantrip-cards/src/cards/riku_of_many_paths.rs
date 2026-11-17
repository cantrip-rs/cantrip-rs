use cantrip_api::*;
use cantrip_mechanics::{effects, library, tokens};
use cantrip_oracle as oracle;

/// The 1/1 blue Bird creature token with flying that Riku's third mode makes.
pub static BIRD: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Bird)
};

pub static EXTRA: &[&CardDef] = &[&BIRD];

/// "Whenever you cast a modal spell" (CR 603.2, 700.2): a spell of yours whose modes were chosen as it was cast (CR 601.2b),
/// with the number of times a mode was chosen for it, one mode twice counting twice (CR 700.2d), in `data[0]` for the ability
/// to choose up to that many (`counted_by_amount`). A spell with no modes chosen is not a modal spell.
fn you_cast_a_modal_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = *event else { return None };
    if player != me.controller {
        return None;
    }
    let (card, modes) = q.history().all().iter().rev().find_map(|r| match r.event {
        Event::ModesChosen { source, ability, modes, .. } if source == spell && ability.index == u8::MAX => Some((ability.card, modes)),
        _ => None,
    })?;
    let chosen = q.card(card).modal?.chosen(modes);
    Some(Fired { subject: Target::None, data: [u32::from(chosen), 0] })
}

/// "Create a 1/1 blue Bird creature token with flying."
fn create_a_bird(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &BIRD, 1)?;
    Ok(())
}

/// "Choose up to X —" (CR 700.2, 603.3c): X is the number of times a mode was chosen for the spell, read as the ability is put
/// on the stack; with none chosen it is removed from the stack. Each mode at most once.
static MODAL: ModalDef = modes(&[
    mode(library::exile_top_card_play_until_end_of_next_turn, "Exile the top card of your library. Until the end of your next turn, you may play it."),
    mode(
        effects::plus_one_counters_on_this_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Trample]).0 }>,
        "Put a +1/+1 counter on Riku. It gains trample until end of turn.",
    ),
    mode(create_a_bird, "Create a 1/1 blue Bird creature token with flying."),
])
.up_to_that_many();

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_modal_spell),
        &MODAL,
        "Whenever you cast a modal spell, choose up to X, where X is the number of times you chose a mode for that spell \u{2014}",
    )
    .counted_by_amount()],
    ..oracle::RIKU_OF_MANY_PATHS
};
