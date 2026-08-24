use std::time::Duration;

use poise::serenity_prelude::{self as serenity, CreateEmbed};

use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const SOURCE_URL: &str =
    "https://kamikouryaku.net/nightreign_eldenring/?%E5%A4%9C%E3%81%AE%E7%8E%8B";
const PANEL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, poise::ChoiceParameter)]
pub enum DayOneBoss {
    #[name = "亜人の女王＆亜人の剣聖"]
    DemiHumanQueens,
    #[name = "鈴玉狩り"]
    BellBearingHunter,
    #[name = "貪食ドラゴン"]
    GapingDragon,
    #[name = "夜の騎兵×2"]
    NightsCavalry,
    #[name = "英雄のガーゴイル"]
    ValiantGargoyle,
    #[name = "ミミズ顔"]
    Wormface,
    #[name = "公のフレイディア"]
    DukesDearFreyja,
    #[name = "百足のデーモン"]
    CentipedeDemon,
    #[name = "熔鉄デーモン"]
    SmelterDemon,
    #[name = "戦場の宿将"]
    BattlefieldCommander,
    #[name = "ティビアの呼び舟"]
    TibiaMariner,
    #[name = "王族の幽鬼"]
    RoyalRevenant,
    #[name = "爛れた樹霊"]
    UlceratedTreeSpirit,
    #[name = "接ぎ木の君主"]
    GraftedMonarch,
    #[name = "傷ついたデーモン＆うろ底のデーモン（DLC）"]
    WoundedDemons,
    #[name = "呪剣士＆神獣の戦士（DLC）"]
    CursebladeAndDivineBeastWarrior,
    #[name = "大赤熊（DLC）"]
    GreatRedBear,
    #[name = "死の騎士（DLC）"]
    DeathKnight,
}

impl DayOneBoss {
    const ALL: [Self; 18] = [
        Self::DemiHumanQueens,
        Self::BellBearingHunter,
        Self::GapingDragon,
        Self::NightsCavalry,
        Self::ValiantGargoyle,
        Self::Wormface,
        Self::DukesDearFreyja,
        Self::CentipedeDemon,
        Self::SmelterDemon,
        Self::BattlefieldCommander,
        Self::TibiaMariner,
        Self::RoyalRevenant,
        Self::UlceratedTreeSpirit,
        Self::GraftedMonarch,
        Self::WoundedDemons,
        Self::CursebladeAndDivineBeastWarrior,
        Self::GreatRedBear,
        Self::DeathKnight,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::DemiHumanQueens => "亜人の女王＆亜人の剣聖",
            Self::BellBearingHunter => "鈴玉狩り",
            Self::GapingDragon => "貪食ドラゴン",
            Self::NightsCavalry => "夜の騎兵×2",
            Self::ValiantGargoyle => "英雄のガーゴイル",
            Self::Wormface => "ミミズ顔",
            Self::DukesDearFreyja => "公のフレイディア",
            Self::CentipedeDemon => "百足のデーモン",
            Self::SmelterDemon => "熔鉄デーモン",
            Self::BattlefieldCommander => "戦場の宿将",
            Self::TibiaMariner => "ティビアの呼び舟",
            Self::RoyalRevenant => "王族の幽鬼",
            Self::UlceratedTreeSpirit => "爛れた樹霊",
            Self::GraftedMonarch => "接ぎ木の君主",
            Self::WoundedDemons => "傷ついたデーモン＆うろ底のデーモン",
            Self::CursebladeAndDivineBeastWarrior => "呪剣士＆神獣の戦士",
            Self::GreatRedBear => "大赤熊",
            Self::DeathKnight => "死の騎士",
        }
    }

    fn is_base_game(self) -> bool {
        !matches!(
            self,
            Self::WoundedDemons
                | Self::CursebladeAndDivineBeastWarrior
                | Self::GreatRedBear
                | Self::DeathKnight
        )
    }

    fn from_value(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|boss| boss.label() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, poise::ChoiceParameter)]
pub enum DayTwoBoss {
    #[name = "ツリーガード＆王都の騎兵"]
    TreeSentinels,
    #[name = "忌み鬼"]
    FellOmen,
    #[name = "古竜"]
    AncientDragon,
    #[name = "僻地の宿将"]
    CommanderONeil,
    #[name = "坩堝の騎士＆黄金カバ"]
    CrucibleKnightAndHippo,
    #[name = "竜のツリーガード＆王都の騎兵"]
    DraconicTreeSentinels,
    #[name = "ノクスの竜人兵"]
    NoxDragonkinSoldier,
    #[name = "大土竜"]
    GreatWyrm,
    #[name = "神肌の貴種＆神肌の使徒"]
    GodskinDuo,
    #[name = "降る星の成獣"]
    FallingstarBeast,
    #[name = "死儀礼の鳥"]
    DeathRiteBird,
    #[name = "無名の王"]
    NamelessKing,
    #[name = "冷たい谷の踊り子"]
    Dancer,
    #[name = "デーモンの王子（DLC）"]
    DemonPrince,
    #[name = "血の君主（DLC）"]
    LordOfBlood,
    #[name = "神獣獅子舞（DLC）"]
    DivineBeastDancingLion,
    #[name = "騎士アルトリウス（DLC）"]
    KnightArtorias,
}

impl DayTwoBoss {
    const ALL: [Self; 17] = [
        Self::TreeSentinels,
        Self::FellOmen,
        Self::AncientDragon,
        Self::CommanderONeil,
        Self::CrucibleKnightAndHippo,
        Self::DraconicTreeSentinels,
        Self::NoxDragonkinSoldier,
        Self::GreatWyrm,
        Self::GodskinDuo,
        Self::FallingstarBeast,
        Self::DeathRiteBird,
        Self::NamelessKing,
        Self::Dancer,
        Self::DemonPrince,
        Self::LordOfBlood,
        Self::DivineBeastDancingLion,
        Self::KnightArtorias,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::TreeSentinels => "ツリーガード＆王都の騎兵",
            Self::FellOmen => "忌み鬼",
            Self::AncientDragon => "古竜",
            Self::CommanderONeil => "僻地の宿将",
            Self::CrucibleKnightAndHippo => "坩堝の騎士＆黄金カバ",
            Self::DraconicTreeSentinels => "竜のツリーガード＆王都の騎兵",
            Self::NoxDragonkinSoldier => "ノクスの竜人兵",
            Self::GreatWyrm => "大土竜",
            Self::GodskinDuo => "神肌の貴種＆神肌の使徒",
            Self::FallingstarBeast => "降る星の成獣",
            Self::DeathRiteBird => "死儀礼の鳥",
            Self::NamelessKing => "無名の王",
            Self::Dancer => "冷たい谷の踊り子",
            Self::DemonPrince => "デーモンの王子",
            Self::LordOfBlood => "血の君主",
            Self::DivineBeastDancingLion => "神獣獅子舞",
            Self::KnightArtorias => "騎士アルトリウス",
        }
    }

    fn is_base_game(self) -> bool {
        !matches!(
            self,
            Self::DemonPrince
                | Self::LordOfBlood
                | Self::DivineBeastDancingLion
                | Self::KnightArtorias
        )
    }

    fn from_value(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|boss| boss.label() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nightlord {
    Gladius,
    Edele,
    Gnoster,
    Maris,
    Libra,
    Fulghor,
    Caligo,
    Nameless,
    Harmonia,
    Strages,
}

impl Nightlord {
    fn label(self) -> &'static str {
        match self {
            Self::Gladius => "三つ首の獣 — 夜の獣、グラディウス",
            Self::Edele => "喰らいつく顎 — 夜の爵、エデレ",
            Self::Gnoster => "知性の蟲 — 夜の識、グノスター",
            Self::Maris => "兆し — 深海の夜、マリス",
            Self::Libra => "調律の魔物 — 夜の魔、リブラ",
            Self::Fulghor => "闇駆ける狩人 — 夜光の騎士、フルゴール",
            Self::Caligo => "霧の裂け目 — 夜の霞、カリゴ",
            Self::Nameless => "夜を象る者 — 夜の王、ナメレス",
            Self::Harmonia => "安寧者たち — 英雄武器の娘たち、ハルモニア",
            Self::Strages => "瓦礫の王 — 反逆のストラゲス",
        }
    }
}

fn in_pair<T: PartialEq>(value: T, choices: &[T]) -> bool {
    choices.contains(&value)
}

fn regular_candidates(day_one: DayOneBoss, day_two: DayTwoBoss) -> Vec<Nightlord> {
    use DayOneBoss as D1;
    use DayTwoBoss as D2;

    let mut result = Vec::new();

    if in_pair(day_one, &[D1::DemiHumanQueens, D1::BellBearingHunter])
        && in_pair(day_two, &[D2::TreeSentinels, D2::FellOmen])
    {
        result.push(Nightlord::Gladius);
    }

    if (in_pair(day_one, &[D1::GapingDragon, D1::DukesDearFreyja])
        && in_pair(day_two, &[D2::AncientDragon, D2::CrucibleKnightAndHippo]))
        || (day_one == D1::Wormface && in_pair(day_two, &[D2::AncientDragon, D2::CommanderONeil]))
        || (day_one == D1::NightsCavalry
            && in_pair(
                day_two,
                &[
                    D2::AncientDragon,
                    D2::CommanderONeil,
                    D2::CrucibleKnightAndHippo,
                ],
            ))
        || (day_one == D1::ValiantGargoyle
            && in_pair(
                day_two,
                &[
                    D2::AncientDragon,
                    D2::CrucibleKnightAndHippo,
                    D2::CommanderONeil,
                ],
            ))
    {
        result.push(Nightlord::Edele);
    }

    if (day_one == D1::SmelterDemon
        && in_pair(
            day_two,
            &[D2::NoxDragonkinSoldier, D2::DraconicTreeSentinels],
        ))
        || (day_one == D1::UlceratedTreeSpirit
            && in_pair(
                day_two,
                &[
                    D2::NoxDragonkinSoldier,
                    D2::DraconicTreeSentinels,
                    D2::GreatWyrm,
                ],
            ))
        || (in_pair(
            day_one,
            &[
                D1::CentipedeDemon,
                D1::BattlefieldCommander,
                D1::TibiaMariner,
            ],
        ) && in_pair(
            day_two,
            &[
                D2::NoxDragonkinSoldier,
                D2::DraconicTreeSentinels,
                D2::GreatWyrm,
            ],
        ))
    {
        result.push(Nightlord::Gnoster);
    }

    if in_pair(
        day_one,
        &[
            D1::GapingDragon,
            D1::Wormface,
            D1::GraftedMonarch,
            D1::SmelterDemon,
            D1::ValiantGargoyle,
        ],
    ) && in_pair(
        day_two,
        &[D2::TreeSentinels, D2::GodskinDuo, D2::FallingstarBeast],
    ) {
        result.push(Nightlord::Maris);
    }

    if (in_pair(
        day_one,
        &[
            D1::TibiaMariner,
            D1::CentipedeDemon,
            D1::BattlefieldCommander,
        ],
    ) && in_pair(
        day_two,
        &[
            D2::CrucibleKnightAndHippo,
            D2::DeathRiteBird,
            D2::GodskinDuo,
        ],
    )) || (day_one == D1::DukesDearFreyja
        && in_pair(day_two, &[D2::CrucibleKnightAndHippo, D2::DeathRiteBird]))
        || (day_one == D1::RoyalRevenant
            && in_pair(
                day_two,
                &[
                    D2::CrucibleKnightAndHippo,
                    D2::DeathRiteBird,
                    D2::GodskinDuo,
                ],
            ))
    {
        result.push(Nightlord::Libra);
    }

    if (day_one == D1::GapingDragon
        && in_pair(day_two, &[D2::NoxDragonkinSoldier, D2::NamelessKing]))
        || (day_one == D1::CentipedeDemon
            && in_pair(
                day_two,
                &[
                    D2::NoxDragonkinSoldier,
                    D2::NamelessKing,
                    D2::CommanderONeil,
                ],
            ))
        || (day_one == D1::NightsCavalry
            && in_pair(day_two, &[D2::NoxDragonkinSoldier, D2::CommanderONeil]))
        || (in_pair(day_one, &[D1::Wormface, D1::RoyalRevenant])
            && in_pair(
                day_two,
                &[
                    D2::NoxDragonkinSoldier,
                    D2::NamelessKing,
                    D2::CommanderONeil,
                ],
            ))
    {
        result.push(Nightlord::Fulghor);
    }

    if (day_one == D1::SmelterDemon && in_pair(day_two, &[D2::DraconicTreeSentinels, D2::Dancer]))
        || (day_one == D1::UlceratedTreeSpirit && in_pair(day_two, &[D2::GodskinDuo, D2::Dancer]))
        || (in_pair(
            day_one,
            &[D1::TibiaMariner, D1::GraftedMonarch, D1::DukesDearFreyja],
        ) && in_pair(
            day_two,
            &[D2::DraconicTreeSentinels, D2::GodskinDuo, D2::Dancer],
        ))
    {
        result.push(Nightlord::Caligo);
    }

    result
}

fn is_nameless_pattern(day_one: DayOneBoss, day_two: DayTwoBoss) -> bool {
    use DayOneBoss as D1;
    use DayTwoBoss as D2;

    match day_one {
        D1::DemiHumanQueens => in_pair(day_two, &[D2::DraconicTreeSentinels, D2::AncientDragon]),
        D1::BellBearingHunter => in_pair(
            day_two,
            &[D2::FellOmen, D2::NoxDragonkinSoldier, D2::FallingstarBeast],
        ),
        D1::GapingDragon => in_pair(day_two, &[D2::AncientDragon, D2::CommanderONeil]),
        D1::NightsCavalry => in_pair(
            day_two,
            &[
                D2::CrucibleKnightAndHippo,
                D2::NoxDragonkinSoldier,
                D2::CommanderONeil,
                D2::FallingstarBeast,
            ],
        ),
        D1::ValiantGargoyle => in_pair(day_two, &[D2::Dancer, D2::DeathRiteBird, D2::GreatWyrm]),
        D1::Wormface => in_pair(
            day_two,
            &[D2::DraconicTreeSentinels, D2::FellOmen, D2::DeathRiteBird],
        ),
        D1::DukesDearFreyja => in_pair(day_two, &[D2::AncientDragon, D2::Dancer]),
        D1::CentipedeDemon => in_pair(day_two, &[D2::FellOmen, D2::NamelessKing]),
        D1::SmelterDemon => day_two == D2::GreatWyrm,
        D1::BattlefieldCommander => in_pair(
            day_two,
            &[
                D2::TreeSentinels,
                D2::FallingstarBeast,
                D2::CrucibleKnightAndHippo,
            ],
        ),
        D1::TibiaMariner => day_two == D2::TreeSentinels,
        D1::RoyalRevenant => in_pair(
            day_two,
            &[D2::GreatWyrm, D2::AncientDragon, D2::NamelessKing],
        ),
        D1::UlceratedTreeSpirit => in_pair(
            day_two,
            &[D2::DeathRiteBird, D2::TreeSentinels, D2::CommanderONeil],
        ),
        D1::GraftedMonarch => in_pair(
            day_two,
            &[
                D2::Dancer,
                D2::GodskinDuo,
                D2::NamelessKing,
                D2::CrucibleKnightAndHippo,
            ],
        ),
        _ => false,
    }
}

fn nightlord_candidates(day_one: DayOneBoss, day_two: DayTwoBoss) -> Vec<Nightlord> {
    use DayOneBoss as D1;
    use DayTwoBoss as D2;

    if in_pair(
        day_one,
        &[D1::WoundedDemons, D1::CursebladeAndDivineBeastWarrior],
    ) && in_pair(day_two, &[D2::DemonPrince, D2::LordOfBlood])
    {
        return vec![Nightlord::Harmonia];
    }

    if in_pair(day_one, &[D1::GreatRedBear, D1::DeathKnight])
        && in_pair(day_two, &[D2::DivineBeastDancingLion, D2::KnightArtorias])
    {
        return vec![Nightlord::Strages];
    }

    if !day_one.is_base_game() || !day_two.is_base_game() {
        return Vec::new();
    }

    let mut result = regular_candidates(day_one, day_two);
    if result.is_empty() || is_nameless_pattern(day_one, day_two) {
        result.push(Nightlord::Nameless);
    }
    result
}

fn day_one_candidates(day_one: DayOneBoss) -> Vec<Nightlord> {
    use DayOneBoss as D1;
    use Nightlord as N;

    let mut result = match day_one {
        D1::DemiHumanQueens | D1::BellBearingHunter => vec![N::Gladius],
        D1::GapingDragon | D1::Wormface => vec![N::Edele, N::Maris, N::Fulghor],
        D1::NightsCavalry => vec![N::Edele, N::Fulghor],
        D1::ValiantGargoyle => vec![N::Edele, N::Maris],
        D1::DukesDearFreyja => vec![N::Edele, N::Libra, N::Caligo],
        D1::CentipedeDemon => vec![N::Gnoster, N::Libra, N::Fulghor],
        D1::SmelterDemon => vec![N::Gnoster, N::Maris, N::Caligo],
        D1::BattlefieldCommander => vec![N::Gnoster, N::Libra],
        D1::TibiaMariner => vec![N::Gnoster, N::Libra, N::Caligo],
        D1::RoyalRevenant => vec![N::Libra, N::Fulghor],
        D1::UlceratedTreeSpirit => vec![N::Gnoster, N::Caligo],
        D1::GraftedMonarch => vec![N::Maris, N::Caligo],
        D1::WoundedDemons | D1::CursebladeAndDivineBeastWarrior => vec![N::Harmonia],
        D1::GreatRedBear | D1::DeathKnight => vec![N::Strages],
    };

    if day_one.is_base_game() {
        result.push(N::Nameless);
    }
    result
}

fn day_two_candidates(day_two: DayTwoBoss) -> Vec<Nightlord> {
    use DayTwoBoss as D2;
    use Nightlord as N;

    let mut result = match day_two {
        D2::TreeSentinels => vec![N::Gladius, N::Maris],
        D2::FellOmen => vec![N::Gladius],
        D2::AncientDragon => vec![N::Edele],
        D2::CommanderONeil => vec![N::Edele, N::Fulghor],
        D2::CrucibleKnightAndHippo => vec![N::Edele, N::Libra],
        D2::DraconicTreeSentinels => vec![N::Gnoster, N::Caligo],
        D2::NoxDragonkinSoldier => vec![N::Gnoster, N::Fulghor],
        D2::GreatWyrm => vec![N::Gnoster],
        D2::GodskinDuo => vec![N::Maris, N::Libra, N::Caligo],
        D2::FallingstarBeast => vec![N::Maris],
        D2::DeathRiteBird => vec![N::Libra],
        D2::NamelessKing => vec![N::Fulghor],
        D2::Dancer => vec![N::Caligo],
        D2::DemonPrince | D2::LordOfBlood => vec![N::Harmonia],
        D2::DivineBeastDancingLion | D2::KnightArtorias => vec![N::Strages],
    };

    if day_two.is_base_game() {
        result.push(N::Nameless);
    }
    result
}

fn selected_candidates(day_one: Option<DayOneBoss>, day_two: Option<DayTwoBoss>) -> Vec<Nightlord> {
    match (day_one, day_two) {
        (Some(day_one), Some(day_two)) => nightlord_candidates(day_one, day_two),
        (Some(day_one), None) => day_one_candidates(day_one),
        (None, Some(day_two)) => day_two_candidates(day_two),
        (None, None) => Vec::new(),
    }
}

fn build_embed(day_one: Option<DayOneBoss>, day_two: Option<DayTwoBoss>) -> CreateEmbed {
    let candidates = selected_candidates(day_one, day_two);
    let (title, color) = match (day_one, day_two, candidates.len()) {
        (None, None, _) => ("🌙 夜の王を逆引き", 0x5865F2),
        (_, _, 0) => ("🌙 該当なし", 0xED4245),
        (_, _, 1) => ("🌙 夜の王", 0x57F287),
        _ => ("🌙 夜の王候補", 0xFEE75C),
    };

    let mut embed = CreateEmbed::new().title(title).url(SOURCE_URL).color(color);

    if let Some(day_one) = day_one {
        embed = embed.field("1日目夜", day_one.label(), true);
    }
    if let Some(day_two) = day_two {
        embed = embed.field("2日目夜", day_two.label(), true);
    }

    match candidates.as_slice() {
        [] if day_one.is_none() && day_two.is_none() => {
            embed.description("1日目または2日目の夜ボスを選択してください。")
        }
        [] => embed.field("結果", "該当する組み合わせがありません。", false),
        [nightlord] => embed.field("3日目", nightlord.label(), false),
        nightlords => embed.field(
            format!("候補（{}体）", nightlords.len()),
            nightlords
                .iter()
                .map(|nightlord| format!("• {}", nightlord.label()))
                .collect::<Vec<_>>()
                .join("\n"),
            false,
        ),
    }
}

fn build_components(
    day_one_id: &str,
    day_two_id: &str,
    reset_id: &str,
    day_one: Option<DayOneBoss>,
    day_two: Option<DayTwoBoss>,
    disabled: bool,
) -> Vec<serenity::CreateActionRow> {
    let day_one_options = DayOneBoss::ALL
        .into_iter()
        .map(|boss| {
            serenity::CreateSelectMenuOption::new(boss.label(), boss.label())
                .default_selection(Some(boss) == day_one)
        })
        .collect();
    let day_two_options = DayTwoBoss::ALL
        .into_iter()
        .map(|boss| {
            serenity::CreateSelectMenuOption::new(boss.label(), boss.label())
                .default_selection(Some(boss) == day_two)
        })
        .collect();

    vec![
        serenity::CreateActionRow::SelectMenu(
            serenity::CreateSelectMenu::new(
                day_one_id,
                serenity::CreateSelectMenuKind::String {
                    options: day_one_options,
                },
            )
            .placeholder(
                day_one
                    .map(|boss| format!("1日目：{}", boss.label()))
                    .unwrap_or_else(|| "1日目の夜ボスを選択".to_string()),
            )
            .disabled(disabled),
        ),
        serenity::CreateActionRow::SelectMenu(
            serenity::CreateSelectMenu::new(
                day_two_id,
                serenity::CreateSelectMenuKind::String {
                    options: day_two_options,
                },
            )
            .placeholder(
                day_two
                    .map(|boss| format!("2日目：{}", boss.label()))
                    .unwrap_or_else(|| "2日目の夜ボスを選択".to_string()),
            )
            .disabled(disabled),
        ),
        serenity::CreateActionRow::Buttons(vec![serenity::CreateButton::new(reset_id)
            .label("リセット")
            .style(serenity::ButtonStyle::Secondary)
            .disabled(disabled || (day_one.is_none() && day_two.is_none()))]),
    ]
}

/// 夜ボスをプルダウンで選び、3日目の夜の王を逆引きします
#[poise::command(slash_command)]
pub async fn nightlord(ctx: Context<'_>) -> Result<(), Error> {
    let session_id = ctx.id();
    let day_one_id = format!("nightlord:{session_id}:day-one");
    let day_two_id = format!("nightlord:{session_id}:day-two");
    let reset_id = format!("nightlord:{session_id}:reset");
    let mut day_one = None;
    let mut day_two = None;

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .embed(build_embed(day_one, day_two))
                .components(build_components(
                    &day_one_id,
                    &day_two_id,
                    &reset_id,
                    day_one,
                    day_two,
                    false,
                )),
        )
        .await?;

    let deadline = tokio::time::Instant::now() + PANEL_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let valid_ids = [day_one_id.clone(), day_two_id.clone(), reset_id.clone()];
        let Some(interaction) = serenity::ComponentInteractionCollector::new(ctx)
            .channel_id(ctx.channel_id())
            .timeout(remaining)
            .filter(move |interaction| valid_ids.contains(&interaction.data.custom_id))
            .await
        else {
            break;
        };

        if interaction.user.id != ctx.author().id {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("この操作パネルはコマンドを実行した本人だけが操作できます。")
                            .ephemeral(true),
                    ),
                )
                .await?;
            continue;
        }

        if interaction.data.custom_id == reset_id {
            day_one = None;
            day_two = None;
        } else if let serenity::ComponentInteractionDataKind::StringSelect { values } =
            &interaction.data.kind
        {
            let value = values.first().map(String::as_str);
            if interaction.data.custom_id == day_one_id {
                day_one = value.and_then(DayOneBoss::from_value);
            } else if interaction.data.custom_id == day_two_id {
                day_two = value.and_then(DayTwoBoss::from_value);
            }
        }

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(build_embed(day_one, day_two))
                        .components(build_components(
                            &day_one_id,
                            &day_two_id,
                            &reset_id,
                            day_one,
                            day_two,
                            false,
                        )),
                ),
            )
            .await?;
    }

    reply
        .edit(
            ctx,
            poise::CreateReply::default().components(build_components(
                &day_one_id,
                &day_two_id,
                &reset_id,
                day_one,
                day_two,
                true,
            )),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_single_regular_nightlord() {
        assert_eq!(
            nightlord_candidates(DayOneBoss::BellBearingHunter, DayTwoBoss::TreeSentinels),
            vec![Nightlord::Gladius]
        );
    }

    #[test]
    fn keeps_all_candidates_for_an_ambiguous_pair() {
        assert_eq!(
            nightlord_candidates(DayOneBoss::NightsCavalry, DayTwoBoss::CommanderONeil),
            vec![Nightlord::Edele, Nightlord::Fulghor, Nightlord::Nameless]
        );
    }

    #[test]
    fn identifies_nameless_from_an_unlisted_regular_pair() {
        assert_eq!(
            nightlord_candidates(DayOneBoss::BellBearingHunter, DayTwoBoss::Dancer),
            vec![Nightlord::Nameless]
        );
    }

    #[test]
    fn identifies_dlc_nightlords() {
        assert_eq!(
            nightlord_candidates(DayOneBoss::WoundedDemons, DayTwoBoss::LordOfBlood),
            vec![Nightlord::Harmonia]
        );
        assert_eq!(
            nightlord_candidates(DayOneBoss::DeathKnight, DayTwoBoss::KnightArtorias),
            vec![Nightlord::Strages]
        );
    }

    #[test]
    fn rejects_a_mixed_base_and_dlc_pair() {
        assert!(nightlord_candidates(DayOneBoss::GapingDragon, DayTwoBoss::DemonPrince).is_empty());
    }

    #[test]
    fn narrows_candidates_from_only_day_one() {
        assert_eq!(
            selected_candidates(Some(DayOneBoss::NightsCavalry), None),
            vec![Nightlord::Edele, Nightlord::Fulghor, Nightlord::Nameless]
        );
    }

    #[test]
    fn narrows_candidates_from_only_day_two() {
        assert_eq!(
            selected_candidates(None, Some(DayTwoBoss::GodskinDuo)),
            vec![
                Nightlord::Maris,
                Nightlord::Libra,
                Nightlord::Caligo,
                Nightlord::Nameless,
            ]
        );
    }

    #[test]
    fn requires_at_least_one_night_boss() {
        assert!(selected_candidates(None, None).is_empty());
    }

    #[test]
    fn component_panel_has_two_selects_and_a_reset_button() {
        let components = build_components("day-one", "day-two", "reset", None, None, false);
        let json = serde_json::to_value(components).expect("components should serialize");

        assert_eq!(json.as_array().map(Vec::len), Some(3));
        assert_eq!(json[0]["components"][0]["type"], 3);
        assert_eq!(
            json[0]["components"][0]["options"].as_array().map(Vec::len),
            Some(18)
        );
        assert_eq!(json[1]["components"][0]["type"], 3);
        assert_eq!(
            json[1]["components"][0]["options"].as_array().map(Vec::len),
            Some(17)
        );
        assert_eq!(json[2]["components"][0]["type"], 2);
    }

    #[test]
    fn select_values_round_trip_to_bosses() {
        for boss in DayOneBoss::ALL {
            assert_eq!(DayOneBoss::from_value(boss.label()), Some(boss));
        }
        for boss in DayTwoBoss::ALL {
            assert_eq!(DayTwoBoss::from_value(boss.label()), Some(boss));
        }
    }

    #[test]
    fn result_embed_does_not_include_the_removed_footer() {
        let embed = build_embed(Some(DayOneBoss::BellBearingHunter), None);
        let json = serde_json::to_value(embed).expect("embed should serialize");

        assert!(json.get("footer").is_none());
    }
}
