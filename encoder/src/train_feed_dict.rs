use blaseball_vcr::feed::{CompactedFeedEvent, FeedEvent};
use clap::clap_app;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;

fn main() {
    let mut feed_samples: Vec<u8> = Vec::new();
    let mut feed_sample_lens: Vec<usize> = Vec::new();

    let mut player_tag_table: HashMap<Uuid, u16> = HashMap::new();
    let mut game_tag_table: HashMap<Uuid, u16> = HashMap::new();
    let mut team_tag_table: HashMap<Uuid, u8> = HashMap::new();

    let matches = clap_app!(train_feed_dict =>
        (version: "1.0")
        (author: "allie signet <allie@sibr.dev>")
        (about: "blaseball.vcr feed zstd dict trainer")
        (@arg INPUT: <INPUT> "input feed dump in NDJSON format")
        (@arg OUT: <OUTPUT> "output dict file")
    )
    .get_matches();

    let out_path = Path::new(matches.value_of("OUT").unwrap());
    let input_path = matches.value_of("INPUT").unwrap();

    let f = File::open(input_path).unwrap();
    let reader = BufReader::new(f);

    for l in reader.lines() {
        let event: FeedEvent = serde_json::from_str(&l.unwrap()).unwrap();
        if event.season == 0 {
            continue;
        }
        let compact_player_tags: Vec<u16> = event
            .player_tags
            .unwrap_or_default()
            .iter()
            .map(|id| {
                if let Some(n) = player_tag_table.get(id) {
                    *n
                } else {
                    let n = player_tag_table.len() as u16;
                    player_tag_table.insert(*id, n);
                    n
                }
            })
            .collect();

        let compact_game_tags: Vec<u16> = event
            .game_tags
            .unwrap_or_default()
            .iter()
            .map(|id| {
                if let Some(n) = game_tag_table.get(id) {
                    *n
                } else {
                    let n = game_tag_table.len() as u16;
                    game_tag_table.insert(*id, n);
                    n
                }
            })
            .collect();

        let compact_team_tags: Vec<u8> = event
            .team_tags
            .unwrap_or_default()
            .iter()
            .map(|id| {
                if let Some(n) = team_tag_table.get(id) {
                    *n
                } else {
                    let n = team_tag_table.len() as u8;
                    team_tag_table.insert(*id, n);
                    n
                }
            })
            .collect();

        let mut ev_bytes = CompactedFeedEvent {
            id: event.id,
            category: event.category,
            day: event.day.try_into().unwrap_or(255),
            created: event.created,
            description: event.description,
            player_tags: compact_player_tags,
            game_tags: compact_game_tags,
            team_tags: compact_team_tags,
            etype: event.etype,
            tournament: event.tournament,
            metadata: event.metadata,
            phase: event.phase,
            season: event.season,
        }
        .encode();
        feed_sample_lens.push(ev_bytes.len());
        feed_samples.append(&mut ev_bytes);
    }

    println!("making dict");
    let dict = zstd::dict::from_continuous(&feed_samples, &feed_sample_lens, 400_000).unwrap();
    let mut feed_dict_f = File::create(out_path).unwrap();
    feed_dict_f.write_all(&dict).unwrap();
    println!("done?");
}
