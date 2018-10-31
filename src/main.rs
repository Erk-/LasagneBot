// TODO: Remove when diesel gets updated!!
#![allow(proc_macro_derive_resolution_fallback)]

#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use] extern crate serenity;

extern crate kankyo;
extern crate chrono;
extern crate rand;
#[macro_use] extern crate diesel;
extern crate typemap;

use serenity::CACHE;
use serenity::framework::StandardFramework;
use serenity::framework::standard::{HelpBehaviour, help_commands};
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::http;

use rand::distributions::{Distribution, Uniform};

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use chrono::prelude::*;

use typemap::Key;

mod schema;
mod models;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;

use models::Comic;

struct DataBaseConn;

impl Key for DataBaseConn {
    type Value = Arc<Mutex<PgConnection>>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn add_fetch(conn: &PgConnection, date: NaiveDate) -> QueryResult<usize>  {
    use schema::comics::dsl::*;
    diesel::insert_into(comics)
        .values(Comic { id: date, fetch_count: 1 })
        .on_conflict(id)
        .do_update()
        .set(fetch_count.eq(fetch_count + 1))
        .execute(conn)
}


fn main() {
    kankyo::load().expect("Failed to load .env file");
    env_logger::init();

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    {
        let mut data = client.data.lock();
        let connection = establish_connection();
        data.insert::<DataBaseConn>(Arc::new(Mutex::new(connection)));
    }

    let owners = match http::get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        },
        Err(why) => panic!("Couldn't get application info: {:?}", why),
    };

    client.with_framework(StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .on_mention(true)
            .prefix("="))
        .customised_help(help_commands::with_embeds, |c| {
            c.individual_command_tip("If you want more information about a command pass it as an argument to help.")
             .lacking_permissions(HelpBehaviour::Hide)
             .striked_commands_tip(None)
             .command_not_found_text("Command not found {}")
             .lacking_role(HelpBehaviour::Hide)
             .wrong_channel(HelpBehaviour::Hide)
        })
        .group("Commands", |g| g
            .command("today", |c| c.cmd(today)
                .desc("todays comic"))
            .command("about", |c| c.cmd(about))
            .command("invite", |c| c.cmd(invite))
            .command("tomorrow", |c| c.cmd(tomorrow))
            .command("yesterday", |c| c.cmd(yesterday))   
            .command("date", |c| c.cmd(other_day)
                .num_args(1)
                .desc("comic from a specific date")
                .usage("yyyy-mm-dd"))
            .command("random", |c| c.cmd(random)
                     .desc("random comic"))
            .command("leaderboard", |c| c.cmd(leaderboard)
                     .known_as("lb")
                     .desc("Shows the most popular daily comics"))
            .command("stats", |c| c.cmd(stats)).owners_only(true)
        ));

    if let Err(why) = client.start() {
        error!("Client error: {:?}", why);
    }
}

command!(yesterday(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let date: NaiveDate = NaiveDate::from_ymd(utc.year(), utc.month(), utc.day()).pred();
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
});

command!(tomorrow(_ctx, msg, _args) {
    let mut rng = rand::thread_rng();
    let time_travel: [&str; 3] = ["2017-08-07", "2015-01-08", "1998-11-14"];
    let range = Uniform::new(0,3);
    let comic_date = range.sample(&mut rng);
    let utc = match NaiveDate::parse_from_str(time_travel[comic_date], "%Y-%m-%d") {
        Ok(day) => day,
        Err(why) => {
            warn!("Error: {}, input: {}", why, comic_date);
            let _ = msg.channel_id.say("Invalid input.");
            return Ok(())
        },
    };
    let _ = match garfield_url(utc) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", utc.year(), utc.month(), utc.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", utc.year(), utc.month(), utc.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date. (date should be between 1978-06-19 and today.)"),
    };
});

command!(invite(_ctx, msg, _args) {
    if let Err(why) =
        msg.channel_id.say("Invite the bot to your server: <https://discordapp.com/oauth2/authorize?client_id=404364579645292564&scope=bot>
") {
        warn!("Error sending message: {:?}", why);
    }
});

command!(stats(_ctx, msg, _args) {
    let guilds = {
        let cache = CACHE.read();
        cache.guilds.clone()
    };
    if let Err(why) = msg.channel_id.say(format!("The bot is in {} servers.", guilds.len())) {
        warn!("Error sending message: {:?}", why);
    }

    let mut svec = Vec::new();

    for g in guilds {
        let pg = g.0.to_partial_guild()?;
        svec.push(pg.name);
    }

    info!("The bot is in the following guilds:\n{:#?}", svec);
});

command!(about(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say("This is a garfield comic bot.") {
        warn!("Error sending message: {:?}", why);
    }
});

fn garfield_url(date: NaiveDate) -> Option<String> {
    //1978-06-19
    let now: DateTime<Utc> = Utc::now();
    let tday: NaiveDate = NaiveDate::from_ymd(now.year(), now.month(), now.day());
    if date > NaiveDate::from_ymd(1978, 6, 18) && date <= tday
    {
        Some(format!("https://d1ejxu6vysztl5.cloudfront.net/comics/garfield/{}/{}-{:02}-{:02}.gif?format=png",
                     date.year(),
                     date.year(),
                     date.month(),
                     date.day()))
    } else {
        None
    }
}

command!(today(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let date: NaiveDate = NaiveDate::from_ymd(utc.year(), utc.month(), utc.day());
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
});

command!(other_day(ctx, msg, args) {
    let date = args.single::<String>().unwrap();
    let utc = match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(day) => day,
        Err(why) => {
            warn!("Error: {}, input: {}", why, date);
            let _ = msg.channel_id.say("Invalid input.");
            return Ok(())
        },
    };

    let data = ctx.data.lock();
    match data.get::<DataBaseConn>() {
        Some(v) => {
            match add_fetch(&v.lock(), utc) {
                Ok(n) => warn!("add_fetch: {}", n),
                Err(err) => warn!("add_fetch err: {:?}", err),
            }
        },
        None => {
            warn!("Could not connect to database");
        },
    };

    let _ = match garfield_url(utc) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", utc.year(), utc.month(), utc.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", utc.year(), utc.month(), utc.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date. (date should be between 1978-06-19 and today.)"),
    };
});

fn get_month_len(month: usize) -> usize {
    match month {
        1 => 31,
        2 => 28,
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 31,
    }
}

command!(random(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let cyear: usize = utc.year() as usize;
    let cmonth: usize = utc.month() as usize;
    let cday: usize = utc.day() as usize;

    let r0 = Uniform::new(1978, cyear+1);
    let r1 = Uniform::new(6, 12+1);
    let r2 = Uniform::new(1, cmonth+1);
    let r3 = Uniform::new(1, 12+1);
    let mut rng = rand::thread_rng();
    let year: usize = r0.sample(& mut rng);
    let month: usize = match year {
        1978 => r1.sample(& mut rng),
        year if year == cyear => r2.sample(&mut rng),
        _ => r3.sample(& mut rng),
    };
    let day: usize = match year {
        1978 => {
            match month {
                6 => Uniform::new(19, 30+1).sample(& mut rng),
                _ => Uniform::new(1, get_month_len(month) + 1).sample(& mut rng),
            }
            },
       year if year == cyear => Uniform::new(1, cday + 1).sample(& mut rng),
        _ => Uniform::new(1, get_month_len(month) + 1).sample(& mut rng),
    };

    let date: NaiveDate = NaiveDate::from_ymd(year as i32, month as u32, day as u32);
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
});

command!(leaderboard(ctx, msg, _args) {
    use schema::comics::dsl::*;
    let data = ctx.data.lock();
    let connection = match data.get::<DataBaseConn>() {
        Some(v) => v.clone(),
        None => {
            let _ = msg.reply("There was a problem getting the database connection");
            return Ok(());
        },
    };
    let ordered_comics: Vec<(NaiveDate, i32)> = match comics
        .order(fetch_count.desc())
        .limit(20)
        .load(&*connection.lock()) {
            Ok(s) => s,
            Err(err) => {
                warn!("Error getting leaderboard: {:?}", err);
                let _ = msg.reply("There was an error getting the leaderboard!");
                return Ok(());
            }
        };

    let mut lb_vec = String::new();
    for c in ordered_comics {
        lb_vec.push_str(&format!("{} | {}\n", c.0, c.1));
    }
    let _ = msg.channel_id.say(&format!("**Leaderboard**\n```Date       | Count\n{}```", lb_vec));
});
