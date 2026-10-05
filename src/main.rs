use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt;
use teloxide::types::{ReactionType, User};
use teloxide::utils::command::{BotCommands, ParseError};

use mate_bot::models::*;
use mate_bot::schema::allowed_chats::dsl::*;
use mate_bot::schema::mates::dsl::*;

const ADMINS: &'static [u64] = &[8322506629];

fn parse_optional_i32(i: String) -> Result<(Option<i32>,), ParseError> {
    match i.trim().parse::<i32>() {
        Ok(i) => Ok((Some(i),)),
        Err(_) => Ok((None,)),
    }
}

#[derive(BotCommands, Clone)]
#[command(
    description = "These commands are supported:",
    rename_rule = "lowercase"
)]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Top statistics", parse_with = parse_optional_i32, alias = "top")]
    Stats(Option<i32>),
    #[command(description = "My statistics")]
    MyStats,
    #[command(description = "Enable chat to count matés")]
    Enable,
    #[command(description = "Fact checked TRUE maté by real argentinian patriots", parse_with = parse_optional_i32)]
    Add(Option<i32>),
    #[command(description = "Remove a FAKE maté", parse_with = parse_optional_i32)]
    Remove(Option<i32>),
    #[command(description = "This bot is licensed AGPL, check out the source code")]
    Source,
    #[command(description = "Turns out there is some of it in maté")]
    Caffeine,
}

pub fn connect() -> SqliteConnection {
    let db_url = env::var("DATABASE_URL").unwrap();
    SqliteConnection::establish(&db_url).unwrap()
}

fn update_mate_count(conn: &mut SqliteConnection, telegram_user: &User, change: i32) {
    let user_id = telegram_user.id.0 as i64;

    let db_user = mates.find(user_id).select(Mates::as_select()).load(conn);

    match db_user {
        Ok(u) if u.len() >= 1 => {
            println!("updating user");
            let u = &u[0];

            diesel::update(mates.find(u.id))
                .set(mate_bot::schema::mates::count.eq(count + change))
                .execute(conn)
                .unwrap();
        }
        _ => {
            println!("creating initial maté");
            let new_data = Mates {
                id: user_id,
                display_name: telegram_user.full_name().to_string(),
                count: change,
            };

            diesel::insert_into(mate_bot::schema::mates::table)
                .values(&new_data)
                .execute(conn)
                .unwrap();
        }
    };
}

async fn photo_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    let conn = &mut connect();

    println!("handling photos");

    let chat = allowed_chats
        .find(msg.chat.id.0 as i64)
        .select(AllowedChats::as_select())
        .load(conn);

    let emoji = if chat.is_ok() && chat.unwrap_or_else(|_| Vec::new()).len() > 0 {
        match &msg.from {
            Some(telegram_user) => {
                update_mate_count(conn, &telegram_user, 1);
                String::from("👍")
            }
            _ => String::from("👎"),
        }
    } else {
        String::from("👎")
    };

    let mut reaction = bot.set_reaction(&msg);

    reaction.reaction = Some(vec![ReactionType::Emoji { emoji: emoji }]);

    reaction.await?;

    Ok(())
}

async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let conn = &mut connect();

    match cmd {
        Command::Help => bot
            .send_message(msg.chat.id, Command::descriptions().to_string())
            .await
            .unwrap(),
        Command::Stats(c) => {
            let mut lb: String = String::new();
            let mut j = 1;

            let c = c.unwrap_or(10);

            for i in mates
                .order_by(mate_bot::schema::mates::count.desc())
                .limit(c as i64)
                .select(Mates::as_select())
                .load(conn)
                .unwrap()
            {
                let line = format!("{j}: {} -- {}\n", i.display_name, i.count);
                lb.push_str(line.as_str());
                j += 1;
            }

            bot.send_message(msg.chat.id, lb).await?
        }
        Command::Enable => {
            if ADMINS.contains(&msg.from.as_ref().unwrap().id.0) {
                let chat = allowed_chats
                    .find(msg.chat.id.0 as i64)
                    .select(AllowedChats::as_select())
                    .load(conn);

                if chat.is_ok() {
                    let new_data = AllowedChats { id: msg.chat.id.0 };

                    diesel::insert_into(mate_bot::schema::allowed_chats::table)
                        .values(&new_data)
                        .execute(conn)
                        .unwrap();

                    bot.send_message(msg.chat.id, format!("Enabled maté count in this channel"))
                        .await?
                } else {
                    bot.send_message(msg.chat.id, format!("Some error i don't understand"))
                        .await?
                }
            } else {
                bot.send_message(msg.chat.id, format!("Not an admin!!"))
                    .await?
            }
        }
        Command::MyStats => {
            let from = &msg.from;

            let user = mates
                .find(from.clone().unwrap().id.0 as i64)
                .select(Mates::as_select())
                .load(conn)
                .unwrap();

            if user.len() > 0 {
                let user = &user[0];

                bot.send_message(msg.chat.id, format!("You have drunk {} matés", user.count))
                    .await?
            } else {
                bot.send_message(msg.chat.id, format!("You have yet to drink an maté"))
                    .await?
            }
        }
        Command::Add(change) => {
            if !ADMINS.contains(&msg.from.as_ref().unwrap().id.0) {
                bot.send_message(msg.chat.id, "Not an admin!!").await?;
                return Ok(());
            }

            let change = change.unwrap_or(1);

            match msg.reply_to_message() {
                Some(replied) => {
                    update_mate_count(conn, replied.from.as_ref().unwrap(), change);

                    let mut reaction = bot.set_reaction(replied);
                    reaction.reaction = Some(vec![ReactionType::Emoji {
                        emoji: String::from("👍"),
                    }]);

                    reaction.await?;

                    bot.send_message(msg.chat.id, format!("Added {} maté to user", change))
                        .await?
                }
                None => {
                    bot.send_message(msg.chat.id, "Please reply to the target user")
                        .await?
                }
            }
        }
        Command::Remove(change) => {
            if !ADMINS.contains(&msg.from.as_ref().unwrap().id.0) {
                bot.send_message(msg.chat.id, "Not an admin!!").await?;
                return Ok(());
            }

            let change = change.unwrap_or(1);

            match msg.reply_to_message() {
                Some(replied) => {
                    update_mate_count(conn, replied.from.as_ref().unwrap(), -change);

                    let mut reaction = bot.set_reaction(replied);
                    reaction.reaction = Some(vec![ReactionType::Emoji {
                        emoji: String::from("👎"),
                    }]);

                    reaction.await?;

                    bot.send_message(msg.chat.id, format!("Removed {} maté from user", change))
                        .await?
                }
                None => {
                    bot.send_message(msg.chat.id, "Please reply to the target user")
                        .await?
                }
            }
        }
        Command::Source => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "See the license for this cool bot over at https://github.com/PadjokeJ/mate_bot"
                ),
            )
            .await?
        }
        Command::Caffeine => {
            let caffeine = mates
                .order_by(mate_bot::schema::mates::count.desc())
                .select(Mates::as_select())
                .load(conn)
                .unwrap()
                .iter()
                .fold(0, |total, m| total + 85 * m.count);

            bot.send_message(msg.chat.id, format!("Using state of the art \"\"maths\"\", I arrive at the conclusion that about {}[mg] of caffeine has been consumed", caffeine)).await?
        }
    };
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let bot = Bot::new(env::var("TELOXIDE_TOKEN").unwrap());

    let cmd = teloxide::filter_command::<Command, _>().endpoint(command_handler);

    let schema = Update::filter_message()
        .branch(cmd)
        .branch(Message::filter_photo().endpoint(photo_handler));

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
