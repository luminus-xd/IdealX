use std::time::Duration;

use poise::serenity_prelude::{self as serenity, CreateEmbed, UserId};
use rand::{seq::SliceRandom, Rng};

use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const SLOT_DELAY: Duration = Duration::from_millis(700);
const PANEL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nightfarer {
    Wylder,
    Guardian,
    Ironeye,
    Duchess,
    Raider,
    Revenant,
    Recluse,
    Executor,
    Scholar,
    Undertaker,
}

impl Nightfarer {
    const BASE_GAME: [Self; 8] = [
        Self::Wylder,
        Self::Guardian,
        Self::Ironeye,
        Self::Duchess,
        Self::Raider,
        Self::Revenant,
        Self::Recluse,
        Self::Executor,
    ];

    const ALL: [Self; 10] = [
        Self::Wylder,
        Self::Guardian,
        Self::Ironeye,
        Self::Duchess,
        Self::Raider,
        Self::Revenant,
        Self::Recluse,
        Self::Executor,
        Self::Scholar,
        Self::Undertaker,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Wylder => "追跡者",
            Self::Guardian => "守護者",
            Self::Ironeye => "鉄の目",
            Self::Duchess => "レディ",
            Self::Raider => "無頼漢",
            Self::Revenant => "復讐者",
            Self::Recluse => "隠者",
            Self::Executor => "執行者",
            Self::Scholar => "学者",
            Self::Undertaker => "葬儀屋",
        }
    }
}

fn candidates(include_dlc: bool) -> &'static [Nightfarer] {
    if include_dlc {
        &Nightfarer::ALL
    } else {
        &Nightfarer::BASE_GAME
    }
}

fn spin<R: Rng + ?Sized>(
    rng: &mut R,
    include_dlc: bool,
    party_size: usize,
    allow_duplicates: bool,
) -> Vec<Nightfarer> {
    let candidates = candidates(include_dlc);
    if allow_duplicates {
        (0..party_size)
            .map(|_| candidates[rng.gen_range(0..candidates.len())])
            .collect()
    } else {
        candidates
            .choose_multiple(rng, party_size)
            .copied()
            .collect()
    }
}

fn pool_label(include_dlc: bool) -> &'static str {
    if include_dlc {
        "全10キャラ"
    } else {
        "本編8キャラ"
    }
}

fn duplicate_label(allow_duplicates: bool) -> &'static str {
    if allow_duplicates {
        "重複あり"
    } else {
        "重複なし"
    }
}

fn build_setup_embed(include_dlc: bool, allow_duplicates: bool, timed_out: bool) -> CreateEmbed {
    let (description, color) = if timed_out {
        (
            "ユーザー選択の受付を終了しました。もう一度 `/nightslot` を実行してください。",
            0x808080,
        )
    } else {
        (
            "下のメニューから、キャラを割り当てるユーザーを**2〜3人**選んでください。\n選択後、自動でスロットが始まります。",
            0x5865F2,
        )
    };

    CreateEmbed::new()
        .title("🎰 ナイトレイン パーティスロット")
        .description(description)
        .color(color)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "抽選対象: {} / {} / 操作できるのはコマンド実行者のみ",
            pool_label(include_dlc),
            duplicate_label(allow_duplicates)
        )))
}

fn build_user_select(custom_id: &str, disabled: bool) -> Vec<serenity::CreateActionRow> {
    vec![serenity::CreateActionRow::SelectMenu(
        serenity::CreateSelectMenu::new(
            custom_id,
            serenity::CreateSelectMenuKind::User {
                default_users: None,
            },
        )
        .placeholder("メンバーを2〜3人選択")
        .min_values(2)
        .max_values(3)
        .disabled(disabled),
    )]
}

fn build_slot_embed(
    players: &[UserId],
    slots: &[Nightfarer],
    stopped: usize,
    include_dlc: bool,
    allow_duplicates: bool,
) -> CreateEmbed {
    debug_assert_eq!(players.len(), slots.len());

    let slot_lines = players
        .iter()
        .zip(slots)
        .enumerate()
        .map(|(index, (player, nightfarer))| {
            let marker = if index < stopped { "🔒" } else { "🔄" };
            format!(
                "{marker} <@{}>　┃ **{}** ┃",
                player.get(),
                nightfarer.label()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let completed = stopped == players.len();
    let title = if completed {
        "🎰 パーティ決定！"
    } else {
        "🎰 ナイトレイン パーティスロット"
    };

    CreateEmbed::new()
        .title(title)
        .description(slot_lines)
        .color(if completed { 0xD4AF37 } else { 0x5865F2 })
        .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
            "抽選対象: {} / {}",
            pool_label(include_dlc),
            duplicate_label(allow_duplicates)
        )))
}

/// ナイトレインの2〜3人パーティをスロットで決めます
#[poise::command(slash_command)]
pub async fn nightslot(
    ctx: Context<'_>,
    #[description = "学者・葬儀屋を抽選に含める（既定: true）"]
    #[rename = "dlc"]
    include_dlc: Option<bool>,
    #[description = "同じキャラの重複を許可する（既定: false）"]
    #[rename = "重複"]
    allow_duplicates: Option<bool>,
) -> Result<(), Error> {
    let include_dlc = include_dlc.unwrap_or(true);
    let allow_duplicates = allow_duplicates.unwrap_or(false);
    let user_select_id = format!("nightslot:{}:users", ctx.id());

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .embed(build_setup_embed(include_dlc, allow_duplicates, false))
                .components(build_user_select(&user_select_id, false)),
        )
        .await?;

    let deadline = tokio::time::Instant::now() + PANEL_TIMEOUT;
    let players = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break None;
        }

        let expected_id = user_select_id.clone();
        let Some(interaction) = serenity::ComponentInteractionCollector::new(ctx)
            .channel_id(ctx.channel_id())
            .timeout(remaining)
            .filter(move |interaction| interaction.data.custom_id == expected_id)
            .await
        else {
            break None;
        };

        if interaction.user.id != ctx.author().id {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("このパネルはコマンドを実行した本人だけが操作できます。")
                            .ephemeral(true),
                    ),
                )
                .await?;
            continue;
        }

        let serenity::ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind
        else {
            continue;
        };
        if !(2..=3).contains(&values.len()) {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("ユーザーを2〜3人選んでください。")
                            .ephemeral(true),
                    ),
                )
                .await?;
            continue;
        }

        let players = values.clone();
        let frames = {
            let mut rng = rand::thread_rng();
            (0..=players.len())
                .map(|_| spin(&mut rng, include_dlc, players.len(), allow_duplicates))
                .collect::<Vec<_>>()
        };
        let result = frames[players.len()].clone();

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(build_slot_embed(
                            &players,
                            &frames[0],
                            0,
                            include_dlc,
                            allow_duplicates,
                        ))
                        .components(Vec::new()),
                ),
            )
            .await?;

        for stopped in 1..=players.len() {
            tokio::time::sleep(SLOT_DELAY).await;

            let mut slots = frames[stopped].clone();
            slots[..stopped].copy_from_slice(&result[..stopped]);
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .embed(build_slot_embed(
                            &players,
                            &slots,
                            stopped,
                            include_dlc,
                            allow_duplicates,
                        ))
                        .components(Vec::new()),
                )
                .await?;
        }

        break Some(players);
    };

    if players.is_none() {
        reply
            .edit(
                ctx,
                poise::CreateReply::default()
                    .embed(build_setup_embed(include_dlc, allow_duplicates, true))
                    .components(build_user_select(&user_select_id, true)),
            )
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn base_game_spin_excludes_dlc_characters() {
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..100 {
            assert!(spin(&mut rng, false, 3, false)
                .into_iter()
                .all(|nightfarer| Nightfarer::BASE_GAME.contains(&nightfarer)));
        }
    }

    #[test]
    fn spin_supports_two_and_three_player_parties() {
        let mut rng = StdRng::seed_from_u64(42);

        assert_eq!(spin(&mut rng, true, 2, false).len(), 2);
        assert_eq!(spin(&mut rng, true, 3, false).len(), 3);
    }

    #[test]
    fn spin_without_duplicates_returns_unique_characters() {
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..100 {
            let party = spin(&mut rng, true, 3, false);
            for (index, nightfarer) in party.iter().enumerate() {
                assert!(!party[..index].contains(nightfarer));
            }
        }
    }

    #[test]
    fn user_select_requires_two_or_three_members() {
        let components = serde_json::to_value(build_user_select("test", false)).unwrap();
        let select = &components[0]["components"][0];

        assert_eq!(select["type"], 5);
        assert_eq!(select["min_values"], 2);
        assert_eq!(select["max_values"], 3);
        assert_eq!(select["disabled"], false);
    }

    #[test]
    fn full_roster_contains_all_ten_characters() {
        assert_eq!(candidates(true).len(), 10);
        assert!(candidates(true).contains(&Nightfarer::Scholar));
        assert!(candidates(true).contains(&Nightfarer::Undertaker));
    }
}
