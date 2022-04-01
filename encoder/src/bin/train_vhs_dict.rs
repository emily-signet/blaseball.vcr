use blaseball_vcr::vhs::recorder::*;
use blaseball_vcr::VCRResult;
use clap::{clap_app, ArgMatches};
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressBar, ProgressStyle};
use new_encoder::*;
use serde_json::value::Value as JSONValue;

#[tokio::main]
pub async fn main() -> VCRResult<()> {
    let matches = clap_app!(train_vhs_dict =>
        (version: "1.0")
        (author: "emily signet <emily@sibr.dev>")
        (about: "blaseball.vcr gen 2 dictionary trainer")
        (@arg CHECKPOINT_EVERY: -c --checkpoints [CHECKPOINT_FREQUENCY] "how often should the diff engine create a checkpoint it can skip to?")
        (@arg OUTPUT: -o --output [FILE] "set output file for dict")
        (@arg ENTITY: <TYPE> ... "entity type to encode")
    )
    .get_matches();

    let etype: String = matches.value_of("ENTITY").unwrap().to_owned();

    use blaseball_vcr::vhs::schemas::*;

    etypes!(
        etype,
        run,
        matches,
        "stadium" > Stadium,
        "communitychestprogress" > CommunityChestProgress,
        "division" > Division,
        "league" > League,
        "playoffmatchup" > Playoffmatchup,
        "playoffround" > Playoffround,
        "playoffs" > Playoffs,
        "season" > Season,
        "sim" > Sim,
        "standings" > Standings,
        "subleague" > Subleague,
        "sunsun" > Sunsun,
        "team" > Team,
        "temporal" > Temporal,
        "tiebreakers" > TiebreakerWrapper,
        "tournament" > Tournament,
        "bonusresult" > Bonusresult,
        "bossfight" > Bossfight,
        "decreeresult" > Decreeresult,
        "eventresult" > Eventresult,
        "fuelprogress" > FuelprogressWrapper,
        "giftprogress" > Giftprogress,
        "globalevents" > GlobaleventsWrapper,
        "idols" > IdolsWrapper,
        "item" > Item,
        "librarystory" > LibrarystoryWrapper,
        "nullified" > NullifiedWrapper,
        "offseasonrecap" > Offseasonrecap,
        "offseasonsetup" > Offseasonsetup,
        "renovationprogress" > Renovationprogress,
        "risingstars" > Risingstars,
        "shopsetup" > Shopsetup,
        "teamelectionstats" > Teamelectionstats,
        "vault" > Vault,
        "player" > Player
    )
    // match etype.as_str() {
    //     "stadium" => run::<Stadium>(etype, matches).await,
    //     "communitychestprogress" => run::<CommunityChestProgress>(etype, matches).await,
    //     "division" => run::<Division>(etype, matches).await,
    //     "league" => run::<League>(etype, matches).await,
    //     "playoffmatchup" => run::<Playoffmatchup>(etype, matches).await,
    //     "playoffround" => run::<Playoffround>(etype, matches).await,
    //     "playoffs" => run::<Playoffs>(etype, matches).await,
    //     "season" => run::<Season>(etype, matches).await,
    //     "sim" => run::<Sim>(etype, matches).await,
    //     "standings" => run::<Standings>(etype, matches).await,
    //     "subleague" => run::<Subleague>(etype, matches).await,
    //     "sunsun" => run::<Sunsun>(etype, matches).await,
    //     "team" => run::<Team>(etype, matches).await,
    //     "temporal" => run::<Temporal>(etype, matches).await,
    //     "tiebreakers" => run::<TiebreakerWrapper>(etype, matches).await,
    //     "tournament" => run::<Tournament>(etype, matches).await,
    //     _ => panic!(),
    // }
}

async fn run<T: vhs_diff::Diff + Clone + serde::de::DeserializeOwned + serde::Serialize>(
    etype: String,
    matches: ArgMatches<'_>,
) -> VCRResult<()> {
    let client = reqwest::Client::new();
    let mut trainer = DictTrainer::new(
        matches
            .value_of("CHECKPOINT_EVERY")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(u16::MAX as usize),
    );

    let bars = MultiProgress::new();
    bars.set_alignment(MultiProgressAlignment::Top);

    let entity_ids: Vec<String> = v2_paged_get(
        &client,
        "https://api.sibr.dev/chronicler/v2/entities",
        &bars,
        ChroniclerParameters {
            next_page: None,
            entity_type: etype.clone(),
            id: None,
            order: None,
            count: 1000,
            before: String::from("2021-09-01T00:00:00Z"),
        },
    )
    .await?
    .into_iter()
    .map(|e| e.entity_id)
    .collect();

    println!("| found {} entities", entity_ids.len());

    let bar_style = ProgressStyle::default_bar()
        .template("{msg:.bold} - {pos}/{len} {wide_bar:40.green/white}")
        .unwrap();

    let entity_id_bar = bars.add(ProgressBar::new(entity_ids.len() as u64));
    entity_id_bar.set_style(bar_style.clone());
    entity_id_bar.set_message("encoding entities");

    for id in entity_id_bar.wrap_iter(entity_ids.into_iter()) {
        entity_id_bar.tick();
        entity_id_bar.set_message(format!("encoding {}", id));

        let entity_versions: Vec<JSONValue> = v2_paged_get(
            &client,
            "https://api.sibr.dev/chronicler/v2/versions",
            &bars,
            ChroniclerParameters {
                next_page: None,
                entity_type: etype.clone(),
                id: Some(id.clone()),
                order: Some("asc".to_owned()),
                count: 1000,
                before: String::from("2021-09-01T00:00:00Z"),
            },
        )
        .await?
        .into_iter()
        .map(|v| v.data)
        .collect();

        if !entity_versions.is_empty() {
            trainer.add_entity(
                entity_versions
                    .into_iter()
                    .map(|v| serde_json::from_value::<T>(v).unwrap())
                    .collect::<Vec<T>>(),
            )?;
        }
    }

    entity_id_bar.finish_with_message("done!");

    println!("training dict");

    let dict = trainer.train(112000)?;

    std::fs::write(matches.value_of("OUTPUT").unwrap(), &dict)?;

    Ok(())
}