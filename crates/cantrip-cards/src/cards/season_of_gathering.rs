use cantrip_api::*;
use cantrip_mechanics::{choose, effects, greatest_power, select};
use cantrip_oracle as oracle;

/// {P} - "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn." The creature is
/// chosen as this mode resolves, not targeted (CR 115.1, 608.2d); it gains both keywords whether or not the counter could be
/// put on it (CR 609.3). Chosen again for each instance of the mode (CR 700.2d), the same creature or another.
fn counter_vigilance_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state().battlefield().iter().copied().filter(|&id| q.controller(id) == you && q.chars(id).is(CardType::Creature)).collect()
    };
    let rule = select::creature_permanents(select::exactly(1));
    let Some(&creature) = choose::select(g, you, item.object, Purpose::PutCounters(CounterKind::PlusOne, 1), creatures, rule)?.first() else {
        return Ok(());
    };
    g.add_counters_together(&[(creature, CounterKind::PlusOne, 1)]);
    g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Vigilance, Keyword::Trample])), Duration::EndOfTurn);
    Ok(())
}

/// {P}{P} - "Choose artifact or enchantment. Destroy all permanents of the chosen type." The type is chosen as this mode
/// resolves (CR 608.2d), and every permanent of it, anyone's and an artifact enchantment either way, is destroyed at once
/// (CR 701.8a, 608.2c); one that can't be destroyed stays.
fn destroy_all_of_the_chosen_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.obj(item.object).card;
    let kind = if choose::one_of(g, item.controller, item.object, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0 {
        CardType::Artifact
    } else {
        CardType::Enchantment
    };
    let doomed: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state().battlefield().iter().copied().filter(|&id| q.chars(id).is(kind)).collect()
    };
    g.destroy_all(&doomed);
    Ok(())
}

/// {P}{P}{P} - "Draw cards equal to the greatest power among creatures you control." (CR 208.1, 107.1b): counted as this
/// mode resolves, after the earlier modes' counters (CR 700.2d); none with no creature.
fn draw_greatest_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = greatest_power::among_creatures_you_control(g, item.controller);
    effects::draw(g, item, n)
}

static MODES: ModalDef = modes(&[
    mode(
        counter_vigilance_and_trample,
        "{P} \u{2014} Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
    )
    .pawprints(1),
    mode(destroy_all_of_the_chosen_type, "{P}{P} \u{2014} Choose artifact or enchantment. Destroy all permanents of the chosen type.")
        .pawprints(2),
    mode(draw_greatest_power, "{P}{P}{P} \u{2014} Draw cards equal to the greatest power among creatures you control.").pawprints(3),
])
.pawprints(5);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODES),
    labels: &["Artifact", "Enchantment"],
    ..oracle::SEASON_OF_GATHERING
};
